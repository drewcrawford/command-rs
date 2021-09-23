use crate::Error;

pub trait ExitStatus {
    ///Checks for non-0 status code
    fn check(&self) -> Result<(),i32>;
    ///Checks for non-0 status code, but wraps the result in Error:StatusError, an error type
    fn check_err(&self) -> Result<(),Error>;
}
impl ExitStatus for std::process::ExitStatus {
    fn check(&self) -> Result<(), i32> {
        match self.code() {
            None => {
                Err(-1)
            }
            Some(0) => {
                Ok(())
            }
            Some(other) => {
                Err(other)
            }
        }
    }
    fn check_err(&self) -> Result<(),Error> {
        self.check().map_err(|e| Error::StatusError(e))
    }
}