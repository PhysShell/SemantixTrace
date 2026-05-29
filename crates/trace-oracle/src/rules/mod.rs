//! Built-in oracle rules.

mod command_must_succeed;
mod no_error_modal_after_command;
mod no_unhandled_exception;
mod screen_must_navigate_forward;
mod validations_pass_before_submit;

pub use command_must_succeed::CommandMustSucceed;
pub use no_error_modal_after_command::NoErrorModalAfterCommand;
pub use no_unhandled_exception::NoUnhandledException;
pub use screen_must_navigate_forward::ScreenMustNavigateForward;
pub use validations_pass_before_submit::ValidationsPassBeforeSubmit;
