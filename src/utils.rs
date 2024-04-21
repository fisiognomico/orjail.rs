use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub fn generate_random_str(size: usize) -> String {
    let rand_str: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .map(char::from)
        .collect();
    rand_str
}
