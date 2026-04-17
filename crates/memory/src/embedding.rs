pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;
    fn dimension(&self) -> usize;
}

pub struct MockEmbedder {
    dim: usize,
}

impl MockEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl Embedder for MockEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut rng = RandSimple(text.len() as u64);
        (0 .. self.dim)
            .map(|_| rng.next_f32() * 2.0 - 1.0)
            .collect()
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

struct RandSimple(u64);

impl RandSimple {
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        (self.0 >> 16) as f32 / 65536.0
    }
}
