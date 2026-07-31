pub trait WordRepository {
    fn get_targets(&self, length: usize) -> Vec<String>;
    fn get_words(&self, length: usize) -> Vec<String>;
    fn unload_words(&self, length: usize);
}
