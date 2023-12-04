pub struct Progress {}

impl Progress {
    pub fn process(&self, mesg: &str) {
        println!(":: {}...", mesg);
    }

    pub fn update(&self, mesg: &str) {
        print!("\r   {}", mesg);
    }
}
