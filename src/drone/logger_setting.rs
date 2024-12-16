use super::RustezeDrone;

use logger::LogLevel;

/* LOGGER HANDLER */
impl RustezeDrone {
    pub fn with_info(&mut self) {
        self.logger.set_displayable(LogLevel::Info as u8);
    }

    pub fn with_debug(&mut self) {
        self.logger.set_displayable(LogLevel::Debug as u8);
    }

    pub fn with_error(&mut self) {
        self.logger.set_displayable(LogLevel::Error as u8);
    }

    pub fn with_warn(&mut self) {
        self.logger.set_displayable(LogLevel::Warn as u8);
    }

    pub fn with_all(&mut self) {
        self.logger.set_displayable(LogLevel::All as u8);
    }

    pub fn with_web_socket(&mut self) {
        self.logger.init_web_socket();
    }
}
