use crate::Usage;

pub enum EnvArgs {}

pub type ArgsComponent = Usage<EnvArgs, Vec<String>>;
