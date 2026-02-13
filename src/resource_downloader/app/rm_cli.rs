use crate::common::cli::program_args::ArgRegistryBuilder;
use crate::common::cli::structs::args_supplier::ArgsSupplier;

#[allow(clippy::upper_case_acronyms)]
pub struct RMCLI;

impl ArgsSupplier for RMCLI {
    fn supply(&mut self, arb: &mut ArgRegistryBuilder) {
        arb.add(
            "p",
            "path",
            "Specify a custom path for the Resource Manager data directory",
        );
    }
}
