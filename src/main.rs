mod cli;

fn main() -> eyre::Result<()> {
    let args = cli::parse();
    dbg!(args);

    Ok(())
}
