use crate::cli::ServeArgs;

pub(crate) async fn run(args: &ServeArgs) -> cycms_core::Result<()> {
    let kernel = cycms_kernel::Kernel::build(Some(&args.config)).await?;
    kernel.serve().await
}
