pub struct RealUserOutput;

#[mockall::automock]
pub trait UserOutput {
    fn print(&self, text: &str);
    fn print_error(&self, text: &str);
}

impl UserOutput for RealUserOutput {
    fn print(&self, text: &str) {
        println!("{}", text);
    }

    fn print_error(&self, text: &str) {
        eprintln!("{}", text);
    }
}
