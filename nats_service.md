# Nats Service by example

```rs
use async_nats::service::ServiceExt;
use futures::StreamExt;


#[tokio::main]
async fn main() -> Result<(), async_nats::Error> {

    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());


    let client = async_nats::connect(nats_url).await?;


    let service = client
        .service_builder()
        .description("A handy min max service")
        .start("minmax", "0.0.1")
        .await?;


    let g = service.group("minmax");


    let mut min = g.endpoint("min").await?;


    let mut max = g.endpoint("max").await?;


    tokio::spawn(async move {
        while let Some(request) = min.next().await {

            let ints = decode_input(&request.message.payload);
            let res = format!("{}", ints.iter().min().unwrap());
            request.respond(Ok(res.into())).await.unwrap();
        }
    });


    tokio::spawn(async move {
        while let Some(request) = max.next().await {
            let ints = decode_input(&request.message.payload);
            let res = format!("{}", ints.iter().max().unwrap());
            request.respond(Ok(res.into())).await.unwrap();
        }
    });



    let input_vec = vec![-1, 2, 100, -2000];
    let input_bytes = serde_json::to_vec(&input_vec).unwrap();


    let min_res = client
        .request("minmax.min", input_bytes.clone().into())
        .await?;


    let max_res = client
        .request("minmax.max", input_bytes.into())
        .await?;


    println!(
        "minimum: {}\nmaximum: {}",
        std::str::from_utf8(&min_res.payload).unwrap(),
        std::str::from_utf8(&max_res.payload).unwrap(),
    );


    Ok(())
}


fn decode_input(raw: &[u8]) -> Vec<i32> {
    serde_json::from_slice(raw).unwrap()
}
```
