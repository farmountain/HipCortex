pub struct AureusBridge {
    loops: usize,
}

impl AureusBridge {
    pub fn new() -> Self {
        Self { loops: 0 }
    }

    pub fn reflexion_loop(&mut self) {
        self.loops += 1;
        println!("[AureusBridge] Reflexion loop running.");
    }

    pub fn loops_run(&self) -> usize {
        self.loops
    }

    pub fn reset(&mut self) {
        self.loops = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_counter() {
        let mut a = AureusBridge::new();
        a.reflexion_loop();
        assert_eq!(a.loops_run(), 1);
    }
}
