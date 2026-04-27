use std::{env::VarError, error::Error, path::PathBuf};

use packet_generator::{
    generators::{self, Generator, GlazeGenerator, WithAddons},
    kdl_parser::{ParserOpts, UnparsedKdl},
};

#[global_allocator]
static GLOBAL: dhat::Alloc = dhat::Alloc;

const PROJECT_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[test]
fn memory_profile_bravefrontier() -> Result<(), Box<dyn Error>> {
    let profile_output = std::env::var("PACKET_GENERATOR_DHAT_PROFILE_FILE");
    let _profiler = match profile_output {
        Ok(name) => dhat::Profiler::builder().file_name(name).build(),
        Err(VarError::NotPresent) => dhat::Profiler::builder().build(),
        Err(e) => return Err(Box::new(e)),
    };

    let path = PathBuf::from_iter(&[PROJECT_DIR, "assets", "all.kdl"]);
    let document = std::fs::read_to_string(&path)?;
    let unparsed_kdl = UnparsedKdl::new(&document, &path);
    let (registry, warnings) = packet_generator::parse_kdl(&[unparsed_kdl], &ParserOpts::default())
        .map_err(miette::Report::from)?;

    warnings.print_warnings_if_any();

    let mut generator = generators::CxxGenerator::new();
    generator.add_addon(GlazeGenerator {});

    let _ = generator.generate(&registry, "main")?;

    Ok(())
}
