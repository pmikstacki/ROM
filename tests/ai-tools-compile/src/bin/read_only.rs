use rom_ai::ReadContext;

pub async fn authorized_reads<R: rom::Resource>(
    context: ReadContext,
    query: rom::Query<R>,
) -> rom::Result<Vec<rom::Snapshot<R>>> {
    let _principal = context.actor();
    let _snapshot = context.read::<R>("host-selected-id").await?;
    context.query(&query).await
}

fn main() {}
