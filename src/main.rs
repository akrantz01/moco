use eyre::WrapErr;

mod cli;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Err(error) = dotenvy::dotenv() {
        if !error.not_found() {
            return Err(error).wrap_err("invalid .env file");
        }
    }

    let args = cli::parse();
    dbg!(args);

    Ok(())
}
