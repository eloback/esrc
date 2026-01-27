use async_nats::jetstream::kv::Store;
use bytes::Bytes;
use governor::nanos::Nanos;
use governor::state::StateStore;

/// A state store implementation using a NATS Key-Value store.
#[derive(Debug, Clone)]
pub struct NatsKVStateStore {
    store: Store,
}

impl NatsKVStateStore {
    /// Cria uma nova instância de NatsKVStateStore.
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl StateStore for NatsKVStateStore {
    type Key = String;

    fn measure_and_replace<T, F, E>(&self, key: &Self::Key, f: F) -> Result<T, E>
    where
        F: Fn(Option<Nanos>) -> Result<(T, Nanos), E>,
    {
        let key = key.to_string();
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

        // Block on the async function within the sync context
        match rt
            .block_on(async { self.store.get(&key).await })
            .expect("kv store get failed")
            .map(|bytes| {
                Nanos::from(u64::from_le_bytes(
                    bytes.as_ref().try_into().expect("invalid nanos bytes"),
                ))
            }) {
            Some(existing_nanos) => {
                let (result, new_nanos) = f(Some(existing_nanos))?;

                // Update the entry in the KV store
                rt.block_on(async {
                    self.store
                        .put(&key, Bytes::from(new_nanos.as_u64().to_le_bytes().to_vec()))
                        .await
                })
                .expect("kv store put failed");

                Ok(result)
            },
            None => {
                let (result, new_nanos) = f(None)?;

                // Create a new entry in the KV store
                rt.block_on(async {
                    self.store
                        .put(&key, Bytes::from(new_nanos.as_u64().to_le_bytes().to_vec()))
                        .await
                })
                .expect("kv store put failed");

                Ok(result)
            },
        }
    }
}

/*
 # Exemplo de uso:
````rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Conecte-se ao servidor NATS (ajuste o URL conforme necessário).
    let client = async_nats::connect("nats://localhost:4222").await?;
    let jetstream = async_nats::jetstream::new(client);

    // Crie ou obtenha o bucket KV para o rate limiter.
    // Configure MaxAge para garantir que chaves inativas expirem (opcional, mas recomendado).
    let kv_store = jetstream
        .create_key_value(kv::Config {
            bucket: "rate_limiter".to_string(),
            max_age: Duration::from_secs(300), // Exemplo: expira chaves após 5 minutos.
            ..Default::default()
        })
        .await?;

    let nats_store = NatsKVStateStore::new(kv_store);

    // Crie o limitador de taxa usando a implementação NatsKVStateStore.
    let quota = Quota::per_second(nonzero!(10u32)); // 10 requisições por segundo
    let clock = governor::clock::DefaultClock::default();
    let lim: RateLimiter<_, _, _, NoOpMiddleware> = RateLimiter::new(quota, nats_store, clock);

    let user_id = "user_123".to_string();

    // Verifique o rate limit
    match lim.check_key(&user_id) {
        Ok(_) => println!("Requisição permitida para {}", user_id),
        Err(_) => {},
    }

    Ok(())
}
```
*/
