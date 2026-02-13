use crate::common::cli::program_args::ArgRegistryBuilder;

pub trait ArgsSupplier: 'static + Send + Sync {
    fn supply(&mut self, arb: &mut ArgRegistryBuilder);
}
