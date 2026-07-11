pub struct RealUserOutput;

#[mockall::automock]
pub trait UserOutput {
    fn print(&self, text: &str);
}

impl UserOutput for RealUserOutput {
    #[allow(clippy::print_stdout)]
    fn print(&self, text: &str) {
        println!("{text}");
    }
}
