// Spawns a bunch of "replicas" and sends same messages to all of them.
// Replicas are connected via fixed-width message queues.
// Using FuturesUnordered, easy to deadlock, as shown [here](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=66455d3e61217d86ec4839c926619bf0).
// Based on real-life bug.

use tokio::sync::mpsc::channel;

#[tokio::main]
async fn main() {
    moro::async_scope!(|scope| {
        // Start up the replicas
        let replicas = 3;
        let mut host_senders = vec![];
        let mut host_futures = vec![];
        for host in 0..replicas {
            let (sender, receiver) = channel(222);
            host_senders.push(sender);
            host_futures.push(scope.spawn(replica(host, receiver)));
        }

        // Send the data
        for message in ['H', 'e', 'l', 'l', 'o', '\n'] {
            for sender in &host_senders {
                sender.send(message).await.unwrap();
            }
        }

        // Drain the replicas.
        for future in host_futures {
            let (host, count) = future.await;
            eprintln!("Host {host} received {count} bytes.");
        }
    })
    .await;

    eprintln!("All done")
}

async fn replica(host: u32, mut receiver: tokio::sync::mpsc::Receiver<char>) -> (u32, usize) {
    let mut count = 0;
    while let Some(message) = receiver.recv().await {
        eprintln!("Host {host} received message {message:?}");
        if message == '\n' {
            break;
        } else {
            count += 1;
        }
    }
    (host, count)
}
