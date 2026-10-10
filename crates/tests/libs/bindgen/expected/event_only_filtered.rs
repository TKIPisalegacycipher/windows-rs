windows_core::imp::define_interface!(
    Handler,
    Handler_Vtbl,
    0xd8ccb2eb_f4d4_55c6_977f_a23ed0a7d401
);
impl windows_core::RuntimeType for Handler {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct Handler_Vtbl {
    base__: windows_core::IUnknown_Vtbl,
    Invoke: unsafe extern "system" fn(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        args: i32,
    ) -> windows_core::HRESULT,
}
struct HandlerBox<F: Fn(windows_core::Ref<IFoo>, i32) + 'static>(
    core::marker::PhantomData<(fn() -> F,)>,
);
impl<F: Fn(windows_core::Ref<IFoo>, i32) + 'static> HandlerBox<F> {
    const VTABLE: Handler_Vtbl = Handler_Vtbl {
        base__: windows_core::IUnknown_Vtbl {
            QueryInterface: windows_core::imp::DelegateBox::<Handler, F>::QueryInterface,
            AddRef: windows_core::imp::DelegateBox::<Handler, F>::AddRef,
            Release: windows_core::imp::DelegateBox::<Handler, F>::Release,
        },
        Invoke: Self::Invoke,
    };
    unsafe extern "system" fn Invoke(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        args: i32,
    ) -> windows_core::HRESULT {
        unsafe {
            let this = &mut *(this as *mut *mut core::ffi::c_void
                as *mut windows_core::imp::DelegateBox<Handler, F>);
            (this.invoke)(core::mem::transmute_copy(&sender), args);
            windows_core::HRESULT(0)
        }
    }
}
windows_core::imp::define_interface!(IFoo, IFoo_Vtbl, 0x785da651_d979_57a5_b98f_6fda69106933);
impl windows_core::RuntimeType for IFoo {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(IFoo, windows_core::IUnknown, windows_core::IInspectable);
impl IFoo {
    pub fn Click<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, i32) + 'static,
    {
        let handler: Handler = {
            let com = windows_core::imp::DelegateBox::<Handler, F>::new(
                &HandlerBox::<F>::VTABLE,
                handler,
            );
            unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
        };
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Click)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(
                self.clone(),
                token__,
                windows_core::Interface::vtable(self).RemoveClick,
            ))
        }
    }
}
impl windows_core::RuntimeName for IFoo {
    const NAME: &'static str = "Test.IFoo";
}
#[repr(C)]
pub struct IFoo_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Click: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows_core::HRESULT,
    pub RemoveClick:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
