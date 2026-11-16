//! A ComboBox's drop-down opens inside the window, not as a window of its own
//! (`keep_drop_down_in_window`, Okojo#2474): for a ComboBox in the first render, one added
//! later, and one whose window changed theme.
//!
//! The app runs in a child process (one Application per process). The parent opens the drop-down
//! over UI Automation, from another process as a screen reader would, with PowerShell's
//! System.Windows.Automation (no UI Automation bindings in this crate), and lists the child's
//! top-level windows through Win32. A windowed drop-down is a `PopupWindowSiteBridge` window.
use super::*;

const TEST: &str =
    "native::winui::tests::combo_box_drop_down::combo_box_drop_downs_open_inside_the_window";
const CHILD: &str = "REACTOR_COMBO_DROP_DOWN_TEST_CHILD";

/// Waits for `Repro.Box`, waits for the app's theme changes, expands it, prints the process's
/// visible top-level window classes (`WINDOWS_OPEN=`), and collapses it.
const CLIENT: &str = r#"
param([int]$ProcessId)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type -Namespace Probe -Name Win -MemberDefinition @'
public delegate bool EnumProc(System.IntPtr hwnd, System.IntPtr lparam);
[System.Runtime.InteropServices.DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc proc, System.IntPtr lparam);
[System.Runtime.InteropServices.DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr hwnd, out uint pid);
[System.Runtime.InteropServices.DllImport("user32.dll")] public static extern bool IsWindowVisible(System.IntPtr hwnd);
[System.Runtime.InteropServices.DllImport("user32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)] public static extern int GetClassName(System.IntPtr hwnd, System.Text.StringBuilder name, int max);
'@
$A = [System.Windows.Automation.AutomationElement]
function Handles {
  $handles = [System.Collections.Generic.List[System.IntPtr]]::new()
  [void][Probe.Win]::EnumWindows({ param($h, $l) $p = 0; [void][Probe.Win]::GetWindowThreadProcessId($h, [ref]$p); if ($p -eq $ProcessId -and [Probe.Win]::IsWindowVisible($h)) { $handles.Add($h) }; $true }, [System.IntPtr]::Zero)
  $handles
}
function Get-WindowClass($h) { $b = [System.Text.StringBuilder]::new(256); [void][Probe.Win]::GetClassName($h, $b, 256); $b.ToString() }
function Find-Box {
  foreach ($h in Handles) {
    $w = try { $A::FromHandle($h) } catch { $null }
    if (-not $w) { continue }
    $e = $w.FindFirst([System.Windows.Automation.TreeScope]::Descendants,
      (New-Object System.Windows.Automation.PropertyCondition($A::AutomationIdProperty, 'Repro.Box')))
    if ($e) { return $e }
  }
  $null
}
$deadline = (Get-Date).AddSeconds(30)
while (-not ($box = Find-Box)) {
  if ((Get-Date) -gt $deadline) { 'ERROR=no Repro.Box'; exit 1 }
  Start-Sleep -Milliseconds 200
}
Start-Sleep -Milliseconds 2500
$expand = $box.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern)
$expand.Expand()
Start-Sleep -Milliseconds 800
"EXPANDED=$($expand.Current.ExpandCollapseState)"
"WINDOWS_OPEN=$((Handles | ForEach-Object { Get-WindowClass $_ }) -join ',')"
$expand.Collapse()
"#;

/// When the ComboBox is created, and what happens before its drop-down opens.
#[derive(Clone, Copy)]
enum Arm {
    /// In the first render.
    First,
    /// Added a second after the first render.
    Late,
    /// In the first render; the window turns Dark, then Light, before the drop-down opens.
    Theme,
}

impl Arm {
    fn name(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Late => "late",
            Self::Theme => "theme",
        }
    }

    fn from_name(name: &str) -> Self {
        match name {
            "late" => Self::Late,
            "theme" => Self::Theme,
            _ => Self::First,
        }
    }
}

#[test]
#[ignore = "requires an interactive WinUI desktop"]
fn combo_box_drop_downs_open_inside_the_window() {
    if let Ok(arm) = std::env::var(CHILD) {
        child(Arm::from_name(&arm));
        return;
    }
    let script = std::env::temp_dir().join(format!(
        "reactor-combo-drop-down-{}.ps1",
        std::process::id()
    ));
    std::fs::write(&script, CLIENT).unwrap();
    let mut windowed = Vec::new();
    for arm in [Arm::First, Arm::Late, Arm::Theme] {
        let mut app = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, arm.name())
            .spawn()
            .unwrap();
        let client = std::process::Command::new("pwsh")
            .args(["-NoProfile", "-NonInteractive", "-File"])
            .arg(&script)
            .args(["-ProcessId", &app.id().to_string()])
            .output()
            .unwrap();
        let out = String::from_utf8_lossy(&client.stdout).into_owned();
        println!("--- {}\n{out}", arm.name());
        eprintln!("{}", String::from_utf8_lossy(&client.stderr));
        assert!(
            app.wait().unwrap().success(),
            "the app ({}) failed",
            arm.name()
        );
        assert!(
            client.status.success(),
            "the client ({}) failed",
            arm.name()
        );
        assert!(
            out.contains("EXPANDED=Expanded"),
            "{} didn't open",
            arm.name()
        );
        windowed.push((arm.name(), out.contains("PopupWindowSiteBridge")));
    }
    let _ = std::fs::remove_file(&script);
    assert_eq!(
        windowed,
        [("first", false), ("late", false), ("theme", false)],
        "(arm, drop-down opened as a window of its own)"
    );
}

fn page(with_box: bool) -> View {
    let mut children = vec![keyed("head", TextBlock::new().text("Head"))];
    if with_box {
        children.push(keyed(
            "box",
            ComboBox::new()
                .automation_id("Repro.Box")
                .automation_name("Room list")
                .items_source(["A", "B", "C", "D", "E", "F"]),
        ));
    }
    children.push(keyed(
        "after",
        Button::new().content(TextBlock::new().text("After")),
    ));
    StackPanel::new().keyed_children(children).into()
}

/// The app: renders the arm's page, adds the ComboBox or changes the theme on its schedule (in
/// 100 ms ticks), and exits after 10 s.
fn child(arm: Arm) {
    let result = App::run_with(move |app| {
        let mut runtime = Runtime::new(WinUiAdapter::default());
        runtime.update(page(!matches!(arm, Arm::Late))).unwrap();
        let root = runtime.graph().root().unwrap();
        let window = runtime.adapter_mut().create_window(root).unwrap();
        window.activate().unwrap();
        let timer = native::DispatcherQueue::GetForCurrentThread()?.CreateTimer()?;
        timer.SetInterval(TimeSpan::try_from(Duration::from_millis(100)).unwrap())?;
        let runtime = RefCell::new(runtime);
        let window = RefCell::new(Some(window));
        let ticks = Cell::new(0u32);
        let app = app.clone();
        let tick = timer.Tick(move |_, _| {
            let now = ticks.get() + 1;
            ticks.set(now);
            let mut runtime = runtime.borrow_mut();
            runtime.dispatch_native_events().unwrap();
            match (arm, now) {
                (Arm::Late, 10) => {
                    runtime.update(page(true)).unwrap();
                }
                (Arm::Theme, 10) => set_theme(&runtime, root, native::ElementTheme::Dark),
                (Arm::Theme, 15) => set_theme(&runtime, root, native::ElementTheme::Light),
                _ => {}
            }
            if now >= 100
                && let Some(window) = window.borrow_mut().take()
            {
                window.close().unwrap();
                app.exit().unwrap();
            }
        })?;
        timer.Start()?;
        Ok((timer, tick))
    });
    result.unwrap();
}

/// Sets the requested theme of the window's root element.
fn set_theme(runtime: &Runtime<WinUiAdapter>, root: ObjectId, theme: native::ElementTheme) {
    runtime
        .adapter()
        .ui_element(root)
        .unwrap()
        .cast::<native::FrameworkElement>()
        .unwrap()
        .SetRequestedTheme(theme)
        .unwrap();
}
