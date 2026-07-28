pub trait WordRepository {
    fn get_words(&self, length: usize) -> Vec<String>;
    fn unload_words(&self, length: usize);
}
