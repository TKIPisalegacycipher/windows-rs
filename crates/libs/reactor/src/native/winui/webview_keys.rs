//! A WebView2's accelerator keys (`ElementRef<WebView2>::request_accelerator_keys`).
//!
//! WinUI's `WebView2` control creates its `CoreWebView2CompositionController` itself and keeps it:
//! its API exposes only the `CoreWebView2`, and it doesn't subscribe the controller's
//! `AcceleratorKeyPressed`, so a key pressed with the focus in the page never reaches the XAML
//! tree (no `KeyDown`, no `KeyboardAccelerator`). The control creates that controller from the
//! environment it is given (`EnsureCoreWebView2Async(CoreWebView2Environment)`), by calling
//! `ICoreWebView2Environment3::CreateCoreWebView2CompositionControllerAsync` on it.
//!
//! So a view whose keys are routed starts its core in an [`EnvironmentProxy`] of its environment:
//! it forwards every call, and wraps that one call's operation in an [`ObservedOperation`], which
//! hands the controller it completes with to the adapter before the control sees it. The adapter
//! then subscribes the controller's `AcceleratorKeyPressed`, an event of the host's, raised before
//! the page sees the key. Nothing is opened toward the page.
//!
//! Without an environment of the app's, the proxy wraps the default one the control would have
//! created, made the same way ([`create_default_environment`]).
use super::*;
use core::ffi::c_void;
use std::mem::{ManuallyDrop, offset_of};
use std::sync::atomic::fence;
use std::thread::ThreadId;
use windows_core::{GUID, IInspectable_Vtbl, IUnknown, IUnknown_Vtbl};
use windows_future::{
    AsyncOperationCompletedHandler, AsyncStatus, IAsyncInfo, IAsyncInfo_Impl, IAsyncOperation,
    IAsyncOperation_Impl,
};

type Controller = native::CoreWebView2CompositionController;
type ControllerOperation = IAsyncOperation<Controller>;

/// Called, on the UI thread, with each controller a control creates in a proxied environment:
/// once, or again after its browser process failed and the control made a new one.
pub(super) type ControllerObserver = Rc<dyn Fn(&Controller)>;

const E_POINTER: HRESULT = HRESULT(0x8000_4003_u32 as i32);
const E_ILLEGAL_METHOD_CALL: HRESULT = HRESULT(0x8000_000E_u32 as i32);
const E_ILLEGAL_DELEGATE_ASSIGNMENT: HRESULT = HRESULT(0x8000_0018_u32 as i32);

/// A value used, and dropped, only on the thread that made it. COM may release the objects that
/// hold one on another thread; the value then leaks rather than drop its `Rc`s there.
pub(super) struct ThreadBound<T> {
    thread: ThreadId,
    value: ManuallyDrop<T>,
}

// SAFETY: the value is reached (`get`) and dropped only on `thread`.
unsafe impl<T> Send for ThreadBound<T> {}
// SAFETY: as above.
unsafe impl<T> Sync for ThreadBound<T> {}

impl<T> ThreadBound<T> {
    pub(super) fn new(value: T) -> Self {
        Self {
            thread: std::thread::current().id(),
            value: ManuallyDrop::new(value),
        }
    }

    /// The value, on the thread that made it.
    pub(super) fn get(&self) -> Option<&T> {
        (std::thread::current().id() == self.thread).then_some(&*self.value)
    }
}

impl<T> Drop for ThreadBound<T> {
    fn drop(&mut self) {
        if std::thread::current().id() == self.thread {
            // SAFETY: dropped once, here, on the thread that made it.
            unsafe { ManuallyDrop::drop(&mut self.value) };
        }
    }
}

/// A proxy of a `CoreWebView2Environment` (its default interface, `inner`) that hands each
/// controller created through it (`ICoreWebView2Environment3`) to `observer`.
///
/// A COM object of two faces: `ICoreWebView2Environment` (also its `IUnknown` and
/// `IInspectable`) and `ICoreWebView2Environment3`, each forwarding every call to `inner`. Any
/// other interface asked for is `inner`'s own, so an interface added in a later WebView2 or
/// WinUI works through it as it would without the proxy. That one rule is bent: `QueryInterface`
/// from such an interface reaches `inner`, not the proxy, so the proxy's identity holds only for
/// the two faces. The control asks the environment it is given for those two
/// (`CreateCoreWebView2CompositionControllerAsync`, `CreateCoreWebView2PointerInfo`) and for
/// `ICoreWebView2EnvironmentInterop`, which `inner` answers.
#[repr(C)]
struct EnvironmentProxy {
    environment: &'static native::ICoreWebView2Environment_Vtbl,
    environment3: &'static native::ICoreWebView2Environment3_Vtbl,
    references: AtomicU32,
    inner: native::ICoreWebView2Environment,
    inner3: native::ICoreWebView2Environment3,
    observer: ThreadBound<ControllerObserver>,
}

/// The byte offsets of the proxy's two faces, the `this` its methods are called with.
const FACE: usize = offset_of!(EnvironmentProxy, environment);
const FACE3: usize = offset_of!(EnvironmentProxy, environment3);

/// A proxy of `environment` (any interface of a `CoreWebView2Environment`) whose controllers go
/// to `observer` as they are created. Must be made on the UI thread, where the control calls it.
pub(super) fn proxy_environment(
    environment: &IUnknown,
    observer: ControllerObserver,
) -> windows_core::Result<IUnknown> {
    let inner = environment.cast::<native::ICoreWebView2Environment>()?;
    let inner3 = environment.cast::<native::ICoreWebView2Environment3>()?;
    let proxy = Box::new(EnvironmentProxy {
        environment: &ENVIRONMENT_VTBL,
        environment3: &ENVIRONMENT3_VTBL,
        references: AtomicU32::new(1),
        inner,
        inner3,
        observer: ThreadBound::new(observer),
    });
    let face = Box::into_raw(proxy).cast::<u8>();
    // SAFETY: `face` is the proxy's first face, with the one reference made above.
    Ok(unsafe { IUnknown::from_raw(face.add(FACE).cast()) })
}

/// The proxy whose face at byte offset `OFFSET` is `this`.
///
/// # Safety
///
/// `this` must be that face of a live proxy.
unsafe fn proxy<'a, const OFFSET: usize>(this: *mut c_void) -> &'a EnvironmentProxy {
    unsafe { &*this.cast::<u8>().sub(OFFSET).cast::<EnvironmentProxy>() }
}

static ENVIRONMENT_VTBL: native::ICoreWebView2Environment_Vtbl =
    native::ICoreWebView2Environment_Vtbl {
        base__: inspectable_vtbl::<FACE>(),
        BrowserVersionString: browser_version_string,
        NewBrowserVersionAvailable: add_new_browser_version_available,
        RemoveNewBrowserVersionAvailable: remove_new_browser_version_available,
        CreateCoreWebView2ControllerAsync: create_controller,
        CreateWebResourceResponse: create_web_resource_response,
    };

static ENVIRONMENT3_VTBL: native::ICoreWebView2Environment3_Vtbl =
    native::ICoreWebView2Environment3_Vtbl {
        base__: inspectable_vtbl::<FACE3>(),
        CreateCoreWebView2CompositionControllerAsync: create_composition_controller,
        CreateCoreWebView2PointerInfo: create_pointer_info,
    };

const fn inspectable_vtbl<const OFFSET: usize>() -> IInspectable_Vtbl {
    IInspectable_Vtbl {
        base: IUnknown_Vtbl {
            QueryInterface: query_interface::<OFFSET>,
            AddRef: add_ref::<OFFSET>,
            Release: release::<OFFSET>,
        },
        GetIids: get_iids::<OFFSET>,
        GetRuntimeClassName: get_runtime_class_name::<OFFSET>,
        GetTrustLevel: get_trust_level::<OFFSET>,
    }
}

unsafe extern "system" fn query_interface<const OFFSET: usize>(
    this: *mut c_void,
    iid: *const GUID,
    interface: *mut *mut c_void,
) -> HRESULT {
    if iid.is_null() || interface.is_null() {
        return E_POINTER;
    }
    // SAFETY: COM calls this on one of the proxy's faces, with valid pointers (checked above).
    unsafe {
        let proxy = proxy::<OFFSET>(this);
        let iid = &*iid;
        let face = if *iid == IUnknown::IID
            || *iid == IInspectable::IID
            || *iid == native::ICoreWebView2Environment::IID
        {
            FACE
        } else if *iid == native::ICoreWebView2Environment3::IID {
            FACE3
        } else {
            return proxy.inner.query(iid, interface);
        };
        proxy.references.fetch_add(1, Ordering::Relaxed);
        *interface = std::ptr::from_ref(proxy)
            .cast::<u8>()
            .cast_mut()
            .add(face)
            .cast();
        HRESULT(0)
    }
}

unsafe extern "system" fn add_ref<const OFFSET: usize>(this: *mut c_void) -> u32 {
    // SAFETY: called on a live face.
    unsafe { proxy::<OFFSET>(this) }
        .references
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

unsafe extern "system" fn release<const OFFSET: usize>(this: *mut c_void) -> u32 {
    // SAFETY: called on a live face; the last reference frees the box made in `proxy_environment`.
    unsafe {
        let proxy = proxy::<OFFSET>(this);
        let remaining = proxy.references.fetch_sub(1, Ordering::Release) - 1;
        if remaining == 0 {
            fence(Ordering::Acquire);
            drop(Box::from_raw(std::ptr::from_ref(proxy).cast_mut()));
        }
        remaining
    }
}

unsafe extern "system" fn get_iids<const OFFSET: usize>(
    this: *mut c_void,
    count: *mut u32,
    values: *mut *mut GUID,
) -> HRESULT {
    // SAFETY: forwarded to `inner` with the caller's pointers.
    unsafe {
        let inner = &proxy::<OFFSET>(this).inner;
        (inner.vtable().base__.GetIids)(inner.as_raw(), count, values)
    }
}

unsafe extern "system" fn get_runtime_class_name<const OFFSET: usize>(
    this: *mut c_void,
    value: *mut *mut c_void,
) -> HRESULT {
    // SAFETY: as `get_iids`.
    unsafe {
        let inner = &proxy::<OFFSET>(this).inner;
        (inner.vtable().base__.GetRuntimeClassName)(inner.as_raw(), value)
    }
}

unsafe extern "system" fn get_trust_level<const OFFSET: usize>(
    this: *mut c_void,
    value: *mut i32,
) -> HRESULT {
    // SAFETY: as `get_iids`.
    unsafe {
        let inner = &proxy::<OFFSET>(this).inner;
        (inner.vtable().base__.GetTrustLevel)(inner.as_raw(), value)
    }
}

unsafe extern "system" fn browser_version_string(
    this: *mut c_void,
    value: *mut *mut c_void,
) -> HRESULT {
    // SAFETY: as `get_iids`.
    unsafe {
        let inner = &proxy::<FACE>(this).inner;
        (inner.vtable().BrowserVersionString)(inner.as_raw(), value)
    }
}

unsafe extern "system" fn add_new_browser_version_available(
    this: *mut c_void,
    handler: *mut c_void,
    token: *mut i64,
) -> HRESULT {
    // SAFETY: as `get_iids`.
    unsafe {
        let inner = &proxy::<FACE>(this).inner;
        (inner.vtable().NewBrowserVersionAvailable)(inner.as_raw(), handler, token)
    }
}

unsafe extern "system" fn remove_new_browser_version_available(
    this: *mut c_void,
    token: i64,
) -> HRESULT {
    // SAFETY: as `get_iids`.
    unsafe {
        let inner = &proxy::<FACE>(this).inner;
        (inner.vtable().RemoveNewBrowserVersionAvailable)(inner.as_raw(), token)
    }
}

unsafe extern "system" fn create_controller(
    this: *mut c_void,
    parent: *mut c_void,
    operation: *mut *mut c_void,
) -> HRESULT {
    // SAFETY: as `get_iids`. A windowed controller (no `Composition`) is not observed: WinUI
    // never creates one.
    unsafe {
        let inner = &proxy::<FACE>(this).inner;
        (inner.vtable().CreateCoreWebView2ControllerAsync)(inner.as_raw(), parent, operation)
    }
}

unsafe extern "system" fn create_web_resource_response(
    this: *mut c_void,
    content: *mut c_void,
    status_code: i32,
    reason_phrase: *mut c_void,
    headers: *mut c_void,
    response: *mut *mut c_void,
) -> HRESULT {
    // SAFETY: as `get_iids`.
    unsafe {
        let inner = &proxy::<FACE>(this).inner;
        (inner.vtable().CreateWebResourceResponse)(
            inner.as_raw(),
            content,
            status_code,
            reason_phrase,
            headers,
            response,
        )
    }
}

/// `CreateCoreWebView2CompositionControllerAsync`, the call WinUI's control makes: `inner`'s
/// operation, handed back observed (`observe`).
unsafe extern "system" fn create_composition_controller(
    this: *mut c_void,
    parent: *mut c_void,
    operation: *mut *mut c_void,
) -> HRESULT {
    // SAFETY: forwarded as `get_iids`; on success `*operation` holds the reference `inner` gave,
    // which is taken here and replaced by one to the observed operation.
    unsafe {
        let proxy = proxy::<FACE3>(this);
        let inner = &proxy.inner3;
        let result = (inner.vtable().CreateCoreWebView2CompositionControllerAsync)(
            inner.as_raw(),
            parent,
            operation,
        );
        if result.is_err() || operation.is_null() || (*operation).is_null() {
            return result;
        }
        let Some(observer) = proxy.observer.get() else {
            return result;
        };
        let created = ControllerOperation::from_raw(*operation);
        *operation = observe(created, Rc::clone(observer)).into_raw();
        result
    }
}

unsafe extern "system" fn create_pointer_info(
    this: *mut c_void,
    value: *mut *mut c_void,
) -> HRESULT {
    // SAFETY: as `get_iids`.
    unsafe {
        let inner = &proxy::<FACE3>(this).inner3;
        (inner.vtable().CreateCoreWebView2PointerInfo)(inner.as_raw(), value)
    }
}

/// What an [`ObservedOperation`] and its handler on the operation it wraps share.
struct Observation {
    state: Mutex<ObservedState>,
    observer: ThreadBound<ControllerObserver>,
}

/// A completion handler. WinRT calls an operation's handler on whichever thread completes it,
/// so a handler may be called, and released, on any thread.
struct AnyThreadHandler(AsyncOperationCompletedHandler<Controller>);

// SAFETY: see above; the handler is only called and cloned, never otherwise used.
unsafe impl Send for AnyThreadHandler {}

#[derive(Default)]
struct ObservedState {
    /// The caller's handler, until it is called.
    handler: Option<AnyThreadHandler>,
    /// The caller set a handler (`SetCompleted` takes only one).
    handler_set: bool,
    /// How the wrapped operation ended, once it has.
    outcome: Option<(AsyncStatus, windows_core::Result<Controller>)>,
}

/// An operation that completes as the one it wraps (`inner`) does, after handing the controller
/// it completes with to an observer.
struct ObservedOperation {
    inner: ControllerOperation,
    observation: Arc<Observation>,
}

implement_decl! {
    impl ObservedOperation as ObservedOperation_Impl: [ControllerOperation, IAsyncInfo]
}

/// `operation`, observed: `observer` gets its controller (on the UI thread, where WebView2
/// completes it) before the caller's handler runs. `operation` itself if it can't be observed.
fn observe(operation: ControllerOperation, observer: ControllerObserver) -> ControllerOperation {
    let observation = Arc::new(Observation {
        state: Mutex::new(ObservedState::default()),
        observer: ThreadBound::new(observer),
    });
    let observed: ControllerOperation = ComObject::new(ObservedOperation {
        inner: operation.clone(),
        observation: Arc::clone(&observation),
    })
    .into_interface();
    let Ok(weak) = observed.downgrade() else {
        return operation;
    };
    // The handler holds the observation and a weak reference to the observed operation, so the
    // wrapped operation (which keeps its handler) doesn't keep the observed one alive.
    let handler = AsyncOperationCompletedHandler::new(
        move |inner: Ref<ControllerOperation>, status: AsyncStatus| {
            let outcome = match (inner.as_ref(), status) {
                (Some(inner), AsyncStatus::Completed) => inner.GetResults(),
                (Some(inner), _) => Err(inner
                    .ErrorCode()
                    .map_or_else(Into::into, windows_core::Error::from)),
                (None, _) => Err(windows_core::Error::empty()),
            };
            if let (Ok(controller), Some(observer)) = (&outcome, observation.observer.get()) {
                observer(controller);
            }
            let handler = {
                let mut state = observation.state.lock().unwrap();
                state.outcome = Some((status, outcome));
                state.handler.take()
            };
            if let (Some(AnyThreadHandler(handler)), Some(observed)) = (handler, weak.upgrade()) {
                handler.Invoke(&observed, status)?;
            }
            Ok(())
        },
    );
    match operation.SetCompleted(&handler) {
        Ok(()) => observed,
        Err(_) => operation,
    }
}

impl IAsyncInfo_Impl for ObservedOperation_Impl {
    fn Id(&self) -> windows_core::Result<u32> {
        self.inner.Id()
    }

    fn Status(&self) -> windows_core::Result<AsyncStatus> {
        match &self.observation.state.lock().unwrap().outcome {
            Some((status, _)) => Ok(*status),
            None => Ok(AsyncStatus::Started),
        }
    }

    fn ErrorCode(&self) -> windows_core::Result<HRESULT> {
        match &self.observation.state.lock().unwrap().outcome {
            Some((_, Err(error))) => Ok(error.code()),
            _ => Ok(HRESULT(0)),
        }
    }

    fn Cancel(&self) -> windows_core::Result<()> {
        self.inner.Cancel()
    }

    fn Close(&self) -> windows_core::Result<()> {
        self.inner.Close()
    }
}

impl IAsyncOperation_Impl<Controller> for ObservedOperation_Impl {
    fn SetCompleted(
        &self,
        handler: Ref<AsyncOperationCompletedHandler<Controller>>,
    ) -> windows_core::Result<()> {
        let handler = handler.ok()?.clone();
        let status = {
            let mut state = self.observation.state.lock().unwrap();
            if state.handler_set {
                return Err(E_ILLEGAL_DELEGATE_ASSIGNMENT.into());
            }
            state.handler_set = true;
            match &state.outcome {
                Some((status, _)) => *status,
                None => {
                    state.handler = Some(AnyThreadHandler(handler));
                    return Ok(());
                }
            }
        };
        // Already complete: a handler set late is called at once, as WinRT's are.
        handler.Invoke(&self.to_interface::<ControllerOperation>(), status)
    }

    fn Completed(&self) -> windows_core::Result<AsyncOperationCompletedHandler<Controller>> {
        self.observation
            .state
            .lock()
            .unwrap()
            .handler
            .as_ref()
            .map(|handler| handler.0.clone())
            .ok_or_else(windows_core::Error::empty)
    }

    fn GetResults(&self) -> windows_core::Result<Controller> {
        match &self.observation.state.lock().unwrap().outcome {
            Some((_, outcome)) => outcome.clone(),
            None => Err(E_ILLEGAL_METHOD_CALL.into()),
        }
    }
}

/// Starts creating the environment WinUI's control creates when it is given none
/// (`WebView2::CreateDefaultCoreEnvironment`): no browser or user data folder (WebView2's own
/// defaults, and the `WEBVIEW2_*` variables, apply), `Language` the first of
/// `ApplicationLanguages.Languages` when there is one. The same options matter: two environments on
/// one user data folder must agree on them.
pub(super) fn create_default_environment()
-> windows_core::Result<IAsyncOperation<native::CoreWebView2Environment>> {
    let options = native::CoreWebView2EnvironmentOptions::new()?;
    let languages = native::ApplicationLanguages::Languages()?;
    if languages.Size()? > 0 {
        options.SetLanguage(&languages.GetAt(0)?.to_string_lossy())?;
    }
    native::CoreWebView2Environment::CreateWithOptionsAsync("", "", &options)
}

/// The key-down an `AcceleratorKeyPressed` reports, as `on_key_down` reports one: `key` its
/// `VirtualKey`, `status` its `PhysicalKeyStatus`, `modifiers` read from the keyboard state.
pub(super) fn key_event_info(
    key: u32,
    status: native::CoreWebView2PhysicalKeyStatus,
    modifiers: InputModifiers,
) -> KeyEventInfo {
    KeyEventInfo {
        key: VirtualKey(key),
        original_key: VirtualKey(key),
        status: PhysicalKeyStatus {
            repeat_count: status.repeat_count,
            scan_code: status.scan_code,
            is_extended: status.is_extended_key != 0,
            is_menu_down: status.is_menu_key_down != 0,
            was_down: status.was_key_down != 0,
            is_released: status.is_key_released != 0,
        },
        modifiers,
    }
}

/// Whether an `AcceleratorKeyPressed` of `kind` is a key-down (`KeyDown`, or `SystemKeyDown`
/// with Alt held), the only kind `on_key` gets.
pub(super) fn is_key_down(kind: native::CoreWebView2KeyEventKind) -> bool {
    kind == native::CoreWebView2KeyEventKind::KeyDown
        || kind == native::CoreWebView2KeyEventKind::SystemKeyDown
}

/// Hands an `AcceleratorKeyPressed` key-down to `on_key`, and marks it handled (kept from the
/// page) when `on_key` says so. Anything it can't read leaves the key to the page.
pub(super) fn press(
    on_key: &RefCell<RoutedCallback<KeyEventInfo>>,
    args: &native::CoreWebView2AcceleratorKeyPressedEventArgs,
) {
    if !args.KeyEventKind().is_ok_and(is_key_down) {
        return;
    }
    let (Ok(key), Ok(status)) = (args.VirtualKey(), args.PhysicalKeyStatus()) else {
        return;
    };
    let modifiers = WinUiAdapter::input_modifiers().unwrap_or(InputModifiers::NONE);
    // Cloned, so `on_key` may request new keys (which replaces it) while it runs.
    let callback = on_key.borrow().clone();
    if callback.call(key_event_info(key, status, modifiers)) {
        _ = args.SetHandled(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_down_reads_as_on_key_down_reports_it() {
        let status = native::CoreWebView2PhysicalKeyStatus {
            repeat_count: 1,
            scan_code: 0x13,
            is_extended_key: 0,
            is_menu_key_down: 0,
            was_key_down: 1,
            is_key_released: 0,
        };
        let info = key_event_info(0x52, status, InputModifiers::CONTROL);
        assert_eq!(info.key, VirtualKey(0x52));
        assert_eq!(info.original_key, VirtualKey(0x52));
        assert_eq!(info.modifiers, InputModifiers::CONTROL);
        assert_eq!(
            info.status,
            PhysicalKeyStatus {
                repeat_count: 1,
                scan_code: 0x13,
                is_extended: false,
                is_menu_down: false,
                was_down: true,
                is_released: false,
            }
        );
    }

    #[test]
    fn only_key_downs_reach_on_key() {
        use native::CoreWebView2KeyEventKind as Kind;
        assert!(is_key_down(Kind::KeyDown));
        assert!(is_key_down(Kind::SystemKeyDown));
        assert!(!is_key_down(Kind(1))); // KeyUp
        assert!(!is_key_down(Kind(3))); // SystemKeyUp
    }

    /// A stand-in `CoreWebView2Environment`: two faces as the proxy's, an `other` object that
    /// answers any other interface, and a composition-controller call that returns `operation`.
    #[repr(C)]
    struct FakeEnvironment {
        environment: &'static native::ICoreWebView2Environment_Vtbl,
        environment3: &'static native::ICoreWebView2Environment3_Vtbl,
        references: AtomicU32,
        other: IUnknown,
        operation: Mutex<Option<ControllerOperation>>,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    unsafe fn fake<'a, const OFFSET: usize>(this: *mut c_void) -> &'a FakeEnvironment {
        unsafe { &*this.cast::<u8>().sub(OFFSET).cast::<FakeEnvironment>() }
    }

    const FAKE: usize = offset_of!(FakeEnvironment, environment);
    const FAKE3: usize = offset_of!(FakeEnvironment, environment3);

    static FAKE_VTBL: native::ICoreWebView2Environment_Vtbl =
        native::ICoreWebView2Environment_Vtbl {
            base__: fake_inspectable::<FAKE>(),
            BrowserVersionString: fake_version,
            NewBrowserVersionAvailable: fake_unused_add,
            RemoveNewBrowserVersionAvailable: fake_unused_token,
            CreateCoreWebView2ControllerAsync: fake_unused3,
            CreateWebResourceResponse: fake_unused6,
        };

    static FAKE3_VTBL: native::ICoreWebView2Environment3_Vtbl =
        native::ICoreWebView2Environment3_Vtbl {
            base__: fake_inspectable::<FAKE3>(),
            CreateCoreWebView2CompositionControllerAsync: fake_create,
            CreateCoreWebView2PointerInfo: fake_pointer_info,
        };

    const fn fake_inspectable<const OFFSET: usize>() -> IInspectable_Vtbl {
        IInspectable_Vtbl {
            base: IUnknown_Vtbl {
                QueryInterface: fake_query::<OFFSET>,
                AddRef: fake_add_ref::<OFFSET>,
                Release: fake_release::<OFFSET>,
            },
            GetIids: fake_iids,
            GetRuntimeClassName: fake_unused1,
            GetTrustLevel: fake_trust,
        }
    }

    unsafe extern "system" fn fake_query<const OFFSET: usize>(
        this: *mut c_void,
        iid: *const GUID,
        interface: *mut *mut c_void,
    ) -> HRESULT {
        unsafe {
            let fake = fake::<OFFSET>(this);
            let iid = &*iid;
            let face = if *iid == IUnknown::IID
                || *iid == IInspectable::IID
                || *iid == native::ICoreWebView2Environment::IID
            {
                FAKE
            } else if *iid == native::ICoreWebView2Environment3::IID {
                FAKE3
            } else {
                return fake.other.query(iid, interface);
            };
            fake.references.fetch_add(1, Ordering::Relaxed);
            *interface = std::ptr::from_ref(fake)
                .cast::<u8>()
                .cast_mut()
                .add(face)
                .cast();
            HRESULT(0)
        }
    }

    unsafe extern "system" fn fake_add_ref<const OFFSET: usize>(this: *mut c_void) -> u32 {
        unsafe { fake::<OFFSET>(this) }
            .references
            .fetch_add(1, Ordering::Relaxed)
            + 1
    }

    unsafe extern "system" fn fake_release<const OFFSET: usize>(this: *mut c_void) -> u32 {
        unsafe {
            let fake = fake::<OFFSET>(this);
            let remaining = fake.references.fetch_sub(1, Ordering::Release) - 1;
            if remaining == 0 {
                fake.calls.lock().unwrap().push("freed");
                drop(Box::from_raw(std::ptr::from_ref(fake).cast_mut()));
            }
            remaining
        }
    }

    unsafe extern "system" fn fake_iids(
        _: *mut c_void,
        count: *mut u32,
        values: *mut *mut GUID,
    ) -> HRESULT {
        unsafe {
            *count = 0;
            *values = std::ptr::null_mut();
        }
        HRESULT(0)
    }

    unsafe extern "system" fn fake_trust(_: *mut c_void, value: *mut i32) -> HRESULT {
        unsafe { *value = 7 };
        HRESULT(0)
    }

    unsafe extern "system" fn fake_version(this: *mut c_void, value: *mut *mut c_void) -> HRESULT {
        unsafe {
            fake::<FAKE>(this).calls.lock().unwrap().push("version");
            *value = std::ptr::null_mut();
        }
        HRESULT(0)
    }

    unsafe extern "system" fn fake_create(
        this: *mut c_void,
        _parent: *mut c_void,
        operation: *mut *mut c_void,
    ) -> HRESULT {
        unsafe {
            let fake = fake::<FAKE3>(this);
            fake.calls.lock().unwrap().push("create");
            match fake.operation.lock().unwrap().take() {
                Some(created) => {
                    *operation = created.into_raw();
                    HRESULT(0)
                }
                None => E_ILLEGAL_METHOD_CALL,
            }
        }
    }

    unsafe extern "system" fn fake_pointer_info(
        this: *mut c_void,
        value: *mut *mut c_void,
    ) -> HRESULT {
        unsafe {
            fake::<FAKE3>(this)
                .calls
                .lock()
                .unwrap()
                .push("pointer info");
            *value = std::ptr::null_mut();
        }
        HRESULT(0)
    }

    unsafe extern "system" fn fake_unused1(_: *mut c_void, _: *mut *mut c_void) -> HRESULT {
        E_ILLEGAL_METHOD_CALL
    }

    unsafe extern "system" fn fake_unused3(
        _: *mut c_void,
        _: *mut c_void,
        _: *mut *mut c_void,
    ) -> HRESULT {
        E_ILLEGAL_METHOD_CALL
    }

    unsafe extern "system" fn fake_unused_add(
        _: *mut c_void,
        _: *mut c_void,
        _: *mut i64,
    ) -> HRESULT {
        E_ILLEGAL_METHOD_CALL
    }

    unsafe extern "system" fn fake_unused_token(_: *mut c_void, _: i64) -> HRESULT {
        E_ILLEGAL_METHOD_CALL
    }

    unsafe extern "system" fn fake_unused6(
        _: *mut c_void,
        _: *mut c_void,
        _: i32,
        _: *mut c_void,
        _: *mut c_void,
        _: *mut *mut c_void,
    ) -> HRESULT {
        E_ILLEGAL_METHOD_CALL
    }

    /// A fake environment whose composition-controller call returns `operation`, the calls it
    /// gets, and its `other` object (any COM object: an `IReference<i32>`).
    fn fake_environment(
        operation: Option<ControllerOperation>,
    ) -> (IUnknown, Arc<Mutex<Vec<&'static str>>>, IUnknown) {
        let other: IInspectable = windows_reference::IReference::from(7).into();
        let other: IUnknown = other.cast().unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let fake = Box::new(FakeEnvironment {
            environment: &FAKE_VTBL,
            environment3: &FAKE3_VTBL,
            references: AtomicU32::new(1),
            other: other.clone(),
            operation: Mutex::new(operation),
            calls: Arc::clone(&calls),
        });
        let face = Box::into_raw(fake).cast::<u8>();
        let environment = unsafe { IUnknown::from_raw(face.add(FAKE).cast()) };
        (environment, calls, other)
    }

    /// An operation already complete with `result`, as a WebView2 operation that finished
    /// before its handler was set.
    struct Ready(windows_core::Result<Controller>);

    implement_decl! {
        impl Ready as Ready_Impl: [ControllerOperation, IAsyncInfo]
    }

    impl Ready_Impl {
        fn status(&self) -> AsyncStatus {
            if self.0.is_ok() {
                AsyncStatus::Completed
            } else {
                AsyncStatus::Error
            }
        }
    }

    impl IAsyncInfo_Impl for Ready_Impl {
        fn Id(&self) -> windows_core::Result<u32> {
            Ok(1)
        }
        fn Status(&self) -> windows_core::Result<AsyncStatus> {
            Ok(self.status())
        }
        fn ErrorCode(&self) -> windows_core::Result<HRESULT> {
            Ok(self
                .0
                .as_ref()
                .err()
                .map_or(HRESULT(0), windows_core::Error::code))
        }
        fn Cancel(&self) -> windows_core::Result<()> {
            Ok(())
        }
        fn Close(&self) -> windows_core::Result<()> {
            Ok(())
        }
    }

    impl IAsyncOperation_Impl<Controller> for Ready_Impl {
        fn SetCompleted(
            &self,
            handler: Ref<AsyncOperationCompletedHandler<Controller>>,
        ) -> windows_core::Result<()> {
            handler
                .ok()?
                .Invoke(&self.to_interface::<ControllerOperation>(), self.status())
        }
        fn Completed(&self) -> windows_core::Result<AsyncOperationCompletedHandler<Controller>> {
            Err(windows_core::Error::empty())
        }
        fn GetResults(&self) -> windows_core::Result<Controller> {
            self.0.clone()
        }
    }

    fn ready(result: windows_core::Result<Controller>) -> ControllerOperation {
        ComObject::new(Ready(result)).into_interface()
    }

    /// Any COM object, typed as a controller: the operations only pass it along.
    fn stand_in_controller() -> Controller {
        let object: IInspectable = windows_reference::IReference::from(42).into();
        unsafe { Controller::from_raw(object.into_raw()) }
    }

    fn identity(object: &impl Interface) -> *mut c_void {
        object.cast::<IUnknown>().unwrap().as_raw()
    }

    fn no_observer() -> ControllerObserver {
        Rc::new(|_: &Controller| {})
    }

    #[test]
    fn the_proxy_answers_for_its_two_faces_and_forwards_their_calls() {
        let (environment, calls, _) = fake_environment(None);
        let proxy = proxy_environment(&environment, no_observer()).unwrap();
        let face = proxy.cast::<native::ICoreWebView2Environment>().unwrap();
        let face3 = proxy.cast::<native::ICoreWebView2Environment3>().unwrap();
        // Both faces are the proxy's, of one identity, not the environment's.
        assert_eq!(identity(&face), proxy.as_raw());
        assert_eq!(identity(&face3), proxy.as_raw());
        assert_ne!(proxy.as_raw(), environment.as_raw());
        assert_eq!(face.cast::<IUnknown>().unwrap(), proxy);
        unsafe {
            let mut value = std::ptr::null_mut();
            let r = (face.vtable().BrowserVersionString)(face.as_raw(), &mut value);
            assert_eq!(r, HRESULT(0));
            let r = (face3.vtable().CreateCoreWebView2PointerInfo)(face3.as_raw(), &mut value);
            assert_eq!(r, HRESULT(0));
            let mut trust = 0;
            let r = (face.vtable().base__.GetTrustLevel)(face.as_raw(), &mut trust);
            assert_eq!((r, trust), (HRESULT(0), 7));
        }
        assert_eq!(*calls.lock().unwrap(), ["version", "pointer info"]);
    }

    #[test]
    fn the_proxy_hands_out_the_environments_own_other_interfaces() {
        let (environment, _, other) = fake_environment(None);
        let proxy = proxy_environment(&environment, no_observer()).unwrap();
        let reference = proxy.cast::<windows_reference::IReference<i32>>().unwrap();
        assert_eq!(reference.Value().unwrap(), 7);
        assert_eq!(identity(&reference), other.as_raw());
    }

    #[test]
    fn the_proxy_and_its_environment_are_freed_with_their_last_reference() {
        let (environment, calls, _) = fake_environment(None);
        let proxy = proxy_environment(&environment, no_observer()).unwrap();
        drop(environment);
        let face3 = proxy.cast::<native::ICoreWebView2Environment3>().unwrap();
        drop(proxy);
        assert!(calls.lock().unwrap().is_empty());
        drop(face3);
        assert_eq!(*calls.lock().unwrap(), ["freed"]);
    }

    /// Creates a controller through `proxy` as WinUI's control does, and records the status its
    /// handler gets and the controller it then reads.
    fn create_through(
        proxy: &IUnknown,
    ) -> (
        windows_core::Result<ControllerOperation>,
        Arc<Mutex<Vec<(AsyncStatus, Option<usize>)>>>,
    ) {
        let face3 = proxy.cast::<native::ICoreWebView2Environment3>().unwrap();
        let mut raw = std::ptr::null_mut();
        let result = unsafe {
            (face3.vtable().CreateCoreWebView2CompositionControllerAsync)(
                face3.as_raw(),
                std::ptr::null_mut(),
                &mut raw,
            )
        };
        let completions = Arc::new(Mutex::new(Vec::new()));
        if result.is_err() {
            return (Err(result.into()), completions);
        }
        let operation = unsafe { ControllerOperation::from_raw(raw) };
        let recorded = Arc::clone(&completions);
        operation
            .SetCompleted(&AsyncOperationCompletedHandler::new(
                move |operation: Ref<ControllerOperation>, status| {
                    let controller = operation
                        .as_ref()
                        .and_then(|operation| operation.GetResults().ok())
                        .map(|controller| controller.as_raw() as usize);
                    recorded.lock().unwrap().push((status, controller));
                    Ok(())
                },
            ))
            .unwrap();
        (Ok(operation), completions)
    }

    #[test]
    fn a_controller_created_through_the_proxy_goes_to_the_observer_before_the_caller() {
        let controller = stand_in_controller();
        let (environment, calls, _) = fake_environment(Some(ready(Ok(controller.clone()))));
        let observed = Rc::new(RefCell::new(Vec::new()));
        let observer: ControllerObserver = {
            let observed = Rc::clone(&observed);
            Rc::new(move |c: &Controller| observed.borrow_mut().push(c.as_raw() as usize))
        };
        let proxy = proxy_environment(&environment, observer).unwrap();
        let (operation, completions) = create_through(&proxy);
        let operation = operation.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["create"]);
        let raw = controller.as_raw() as usize;
        assert_eq!(*observed.borrow(), [raw]);
        assert_eq!(
            *completions.lock().unwrap(),
            [(AsyncStatus::Completed, Some(raw))]
        );
        assert_eq!(operation.Status().unwrap(), AsyncStatus::Completed);
        assert_eq!(operation.GetResults().unwrap(), controller);
        // A handler takes one setting, as WinRT's operations do.
        let again = AsyncOperationCompletedHandler::new(|_, _| Ok(()));
        assert_eq!(
            operation.SetCompleted(&again).unwrap_err().code(),
            E_ILLEGAL_DELEGATE_ASSIGNMENT
        );
    }

    #[test]
    fn a_failed_creation_reaches_the_caller_and_not_the_observer() {
        let failure = windows_core::Error::from(HRESULT(0x8007_0002_u32 as i32));
        let (environment, _, _) = fake_environment(Some(ready(Err(failure.clone()))));
        let observed = Rc::new(Cell::new(0));
        let observer: ControllerObserver = {
            let observed = Rc::clone(&observed);
            Rc::new(move |_: &Controller| observed.set(observed.get() + 1))
        };
        let proxy = proxy_environment(&environment, observer).unwrap();
        let (operation, completions) = create_through(&proxy);
        let operation = operation.unwrap();
        assert_eq!(observed.get(), 0);
        assert_eq!(*completions.lock().unwrap(), [(AsyncStatus::Error, None)]);
        assert_eq!(operation.ErrorCode().unwrap(), failure.code());
        assert_eq!(operation.GetResults().unwrap_err().code(), failure.code());
    }

    #[test]
    fn an_error_from_the_environment_is_returned_as_it_is() {
        let (environment, _, _) = fake_environment(None);
        let proxy = proxy_environment(&environment, no_observer()).unwrap();
        let (operation, _) = create_through(&proxy);
        assert_eq!(operation.unwrap_err().code(), E_ILLEGAL_METHOD_CALL);
    }

    #[test]
    fn a_value_bound_to_its_thread_is_reached_only_there() {
        let bound = Arc::new(ThreadBound::new(Rc::new(5)));
        assert_eq!(bound.get().map(|value| **value), Some(5));
        let elsewhere = Arc::clone(&bound);
        let reached = std::thread::spawn(move || elsewhere.get().is_some())
            .join()
            .unwrap();
        assert!(!reached);
    }
}
