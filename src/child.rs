use std::io::Result;

pub struct Child(std::process::Child);

impl Child {
    pub(crate) fn new(process_child: std::process::Child) -> Child {
        Child(process_child)
    }

    // pub async fn wait() -> Result<ExitStatus> {
    //
    // }
}