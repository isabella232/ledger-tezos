pub use bolos_common::hash::{Hasher, HasherId};

mod blake2b;
pub use blake2b::Blake2b;

mod sha256;
pub use sha256::Sha256;

mod sha512;
pub use sha512::Sha512;