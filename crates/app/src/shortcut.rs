//! App-owned global-shortcut registration boundary.

/// Carbon modifier masks used by `RegisterEventHotKey`.
pub const SHIFT: u32 = 1 << 9;
pub const CONTROL: u32 = 1 << 12;
pub const OPTION: u32 = 1 << 11;
pub const COMMAND: u32 = 1 << 8;

/// A user-visible shortcut and its native key representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Shortcut {
    pub key_code: u32,
    pub modifiers: u32,
    pub display_name: String,
}

impl Shortcut {
    pub fn new(key_code: u32, modifiers: u32, display_name: impl Into<String>) -> Self {
        Self {
            key_code,
            modifiers,
            display_name: display_name.into(),
        }
    }

    pub fn launcher_default() -> Self {
        // kVK_Space, controlKey | optionKey.
        Self::new(49, CONTROL | OPTION, "⌃⌥Space")
    }

    pub fn validate(&self) -> Result<(), ShortcutError> {
        // kVK values for left/right command, option, control, shift, caps
        // lock, and function. A modifier cannot form a launcher chord alone.
        if matches!(self.key_code, 54..=63) || self.modifiers & (CONTROL | OPTION | COMMAND) == 0 {
            return Err(ShortcutError::InvalidConfiguration);
        }
        Ok(())
    }

    pub fn is_same_chord(&self, other: &Self) -> bool {
        self.key_code == other.key_code && self.modifiers == other.modifiers
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortcutError {
    InvalidConfiguration,
    Unavailable,
}

impl std::fmt::Display for ShortcutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfiguration => {
                write!(f, "Use Command, Control, or Option with another key.")
            }
            Self::Unavailable => write!(
                f,
                "That shortcut could not be registered. The previous shortcut remains active."
            ),
        }
    }
}

/// Owns one operating-system shortcut registration.
pub trait RegisteredShortcut: 'static {}

/// Allows tests to simulate registration failure without pretending it is a
/// native behavior test.
pub trait ShortcutRegistrar: 'static {
    fn register(
        &mut self,
        shortcut: Shortcut,
    ) -> Result<Box<dyn RegisteredShortcut>, ShortcutError>;
}

/// Updates the native registration transactionally: a failed candidate cannot
/// disturb the active shortcut.
pub struct ShortcutController<R: ShortcutRegistrar> {
    registrar: R,
    active: Option<Box<dyn RegisteredShortcut>>,
    shortcut: Shortcut,
}

impl<R: ShortcutRegistrar> ShortcutController<R> {
    pub fn new(registrar: R) -> Self {
        Self {
            registrar,
            active: None,
            shortcut: Shortcut::launcher_default(),
        }
    }

    pub fn register(&mut self, candidate: Shortcut) -> Result<(), ShortcutError> {
        candidate.validate()?;
        if self.active.is_some()
            && candidate.key_code == self.shortcut.key_code
            && candidate.modifiers == self.shortcut.modifiers
        {
            self.shortcut = candidate;
            return Ok(());
        }
        let registered = self.registrar.register(candidate.clone())?;
        self.active = Some(registered);
        self.shortcut = candidate;
        Ok(())
    }

    pub fn shortcut(&self) -> Shortcut {
        self.shortcut.clone()
    }

    pub fn is_registered(&self) -> bool {
        self.active.is_some()
    }

    pub fn registrar_mut(&mut self) -> &mut R {
        &mut self.registrar
    }
}

/// Production Carbon registration. Carbon delivers hot keys while another
/// application is active, unlike a GPUI key binding confined to this window.
#[cfg(target_os = "macos")]
pub mod macos {
    use std::ffi::c_void;
    use std::sync::{Arc, Mutex, OnceLock};

    use super::{RegisteredShortcut, Shortcut, ShortcutError, ShortcutRegistrar};

    // HIToolbox/CarbonEvents.h: kEventClassKeyboard and kEventHotKeyPressed.
    // kEventHotKeyReleased is 6; subscribing to it misses press-only delivery.
    const KEYBOARD_EVENT_CLASS: u32 = u32::from_be_bytes(*b"keyb");
    const HOT_KEY_PRESSED_EVENT: u32 = 5;

    type OsStatus = i32;
    type EventTargetRef = *mut c_void;
    type EventHandlerRef = *mut c_void;
    type EventHotKeyRef = *mut c_void;
    type EventHandlerCallRef = *mut c_void;
    type EventRef = *mut c_void;

    #[repr(C)]
    struct EventTypeSpec {
        event_class: u32,
        event_kind: u32,
    }

    #[repr(C)]
    struct EventHotKeyId {
        signature: u32,
        id: u32,
    }

    type EventHandler =
        unsafe extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> OsStatus;

    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn GetApplicationEventTarget() -> EventTargetRef;
        fn InstallEventHandler(
            target: EventTargetRef,
            handler: EventHandler,
            number_of_types: u32,
            event_types: *const EventTypeSpec,
            user_data: *mut c_void,
            out_ref: *mut EventHandlerRef,
        ) -> OsStatus;
        fn RegisterEventHotKey(
            key_code: u32,
            modifiers: u32,
            hot_key_id: EventHotKeyId,
            target: EventTargetRef,
            options: u32,
            out_ref: *mut EventHotKeyRef,
        ) -> OsStatus;
        fn UnregisterEventHotKey(hot_key: EventHotKeyRef) -> OsStatus;
    }

    type HotKeyCallback = Arc<dyn Fn() + Send + Sync>;
    static CALLBACK: OnceLock<Mutex<Option<HotKeyCallback>>> = OnceLock::new();
    static HANDLER: OnceLock<()> = OnceLock::new();

    unsafe extern "C" fn hot_key_pressed(
        _call: EventHandlerCallRef,
        _event: EventRef,
        _user_data: *mut c_void,
    ) -> OsStatus {
        if let Some(callback) = CALLBACK
            .get()
            .and_then(|callback| callback.lock().ok())
            .and_then(|callback| callback.as_ref().cloned())
        {
            callback();
        }
        0
    }

    /// One app-local registrar backed by Carbon's global hot-key API.
    pub struct CarbonShortcutRegistrar {
        callback: Arc<dyn Fn() + Send + Sync>,
    }

    impl CarbonShortcutRegistrar {
        pub fn new(callback: impl Fn() + Send + Sync + 'static) -> Self {
            Self {
                callback: Arc::new(callback),
            }
        }

        fn install_handler(&self) -> Result<(), ShortcutError> {
            let callback = CALLBACK.get_or_init(|| Mutex::new(None));
            *callback.lock().map_err(|_| ShortcutError::Unavailable)? = Some(self.callback.clone());
            if HANDLER.get().is_some() {
                return Ok(());
            }
            let event_type = EventTypeSpec {
                event_class: KEYBOARD_EVENT_CLASS,
                event_kind: HOT_KEY_PRESSED_EVENT,
            };
            let mut handler = std::ptr::null_mut();
            let result = unsafe {
                InstallEventHandler(
                    GetApplicationEventTarget(),
                    hot_key_pressed,
                    1,
                    &event_type,
                    std::ptr::null_mut(),
                    &mut handler,
                )
            };
            if result != 0 || handler.is_null() {
                return Err(ShortcutError::Unavailable);
            }
            let _ = HANDLER.set(());
            Ok(())
        }
    }

    impl ShortcutRegistrar for CarbonShortcutRegistrar {
        fn register(
            &mut self,
            shortcut: Shortcut,
        ) -> Result<Box<dyn RegisteredShortcut>, ShortcutError> {
            self.install_handler()?;
            let mut reference = std::ptr::null_mut();
            let status = unsafe {
                RegisterEventHotKey(
                    shortcut.key_code,
                    shortcut.modifiers,
                    EventHotKeyId {
                        signature: u32::from_be_bytes(*b"SDTL"),
                        id: 1,
                    },
                    GetApplicationEventTarget(),
                    0,
                    &mut reference,
                )
            };
            if status != 0 || reference.is_null() {
                return Err(ShortcutError::Unavailable);
            }
            Ok(Box::new(CarbonRegistration(reference)))
        }
    }

    struct CarbonRegistration(EventHotKeyRef);
    impl RegisteredShortcut for CarbonRegistration {}
    impl Drop for CarbonRegistration {
        fn drop(&mut self) {
            unsafe { UnregisterEventHotKey(self.0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeRegistrar {
        rejected: bool,
        registrations: Vec<Shortcut>,
    }

    impl ShortcutRegistrar for FakeRegistrar {
        fn register(
            &mut self,
            shortcut: Shortcut,
        ) -> Result<Box<dyn RegisteredShortcut>, ShortcutError> {
            self.registrations.push(shortcut);
            if self.rejected {
                Err(ShortcutError::Unavailable)
            } else {
                Ok(Box::new(FakeRegistration))
            }
        }
    }

    struct FakeRegistration;
    impl RegisteredShortcut for FakeRegistration {}

    #[test]
    fn the_default_is_control_option_space() {
        assert_eq!(Shortcut::launcher_default().display_name, "⌃⌥Space");
        assert_eq!(Shortcut::launcher_default().key_code, 49);
    }

    #[test]
    fn failed_replacement_keeps_the_last_working_shortcut() {
        let mut controller = ShortcutController::new(FakeRegistrar::default());
        assert!(controller.register(Shortcut::launcher_default()).is_ok());
        let previous = controller.shortcut();
        controller.registrar_mut().rejected = true;

        let error = controller
            .register(Shortcut::new(0, CONTROL, "⌃A"))
            .unwrap_err();

        assert_eq!(error, ShortcutError::Unavailable);
        assert_eq!(controller.shortcut(), previous);
        assert!(controller.is_registered());
    }

    #[test]
    fn configured_shortcut_requires_a_non_modifier_key_and_primary_modifier() {
        for key_code in [54, 55, 56, 57, 58, 59, 60, 61, 62, 63] {
            assert!(Shortcut::new(key_code, CONTROL, "modifier")
                .validate()
                .is_err());
        }
        assert!(Shortcut::new(0, 0, "A").validate().is_err());
        assert!(Shortcut::new(0, CONTROL, "⌃A").validate().is_ok());
    }

    #[test]
    fn registering_the_active_chord_is_a_no_op() {
        let mut controller = ShortcutController::new(FakeRegistrar::default());
        let shortcut = Shortcut::launcher_default();
        controller.register(shortcut.clone()).unwrap();
        controller.registrar_mut().rejected = true;

        controller.register(shortcut.clone()).unwrap();

        assert_eq!(controller.registrar_mut().registrations.len(), 1);
    }

    #[test]
    fn relabelling_the_active_chord_does_not_attempt_self_registration() {
        let mut controller = ShortcutController::new(FakeRegistrar::default());
        let shortcut = Shortcut::launcher_default();
        controller.register(shortcut).unwrap();
        controller.registrar_mut().rejected = true;

        controller
            .register(Shortcut::new(49, CONTROL | OPTION, "Control-Option-Space"))
            .unwrap();

        assert_eq!(controller.shortcut().display_name, "Control-Option-Space");
        assert_eq!(controller.registrar_mut().registrations.len(), 1);
    }
}
