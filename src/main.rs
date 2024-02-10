use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Task {
    id: u32,
    payload: String,
}

pub struct Worker {
    id: u32,
}

impl Worker {
    pub fn process_task(&self, task: Task) -> String {
        // Simulate processing time
        let processing_time = thread_rng().gen_range(0..=3);
        thread::sleep(Duration::from_secs(processing_time));

        "Processed ".to_string() + &task.payload
    }
}

pub fn create_task(id: u32, payload: &str) -> Task {
    Task {
        id,
        payload: payload.to_string(),
    }
}

pub fn create_worker(id: u32) -> Worker {
    Worker { id }
}

enum MessageType {
    INIT,
    FINAL,
}
fn main() {
    let num_tasks: u32 = 10;

    let mut tasks: Vec<Task> = Vec::new();

    for i in 1..=num_tasks {
        tasks.push(Task {
            id: i,
            payload: format!("Task {}", i),
        });
    }

    // Message-passing channels
    let (sender, receiver) = mpsc::channel::<Task>();
    let (sender_resp, receiver_resp) = mpsc::channel();

    // Creating a reference-counted, thread-safe wrapper for multiple threads to share ownership
    let receiver = Arc::new(Mutex::new(receiver));

    let num_workers: u32 = 4; // Number of worker threads
    let mut worker_handles: Vec<thread::JoinHandle<()>> = vec![];
    // Spawn worker threads
    for i in 0..num_workers {
        let receiver_clone = Arc::clone(&receiver);
        let sender_resp_clone = sender_resp.clone();

        let handle = thread::spawn(move || {
            // Worker thread loop
            loop {
                // Panics: if failed to acquire lock
                let _reciever = receiver_clone.lock().unwrap();
                match _reciever.recv() {
                    Ok(task) => {
                        drop(_reciever);

                        let worker = create_worker(i);

                        // Send initialization message
                        sender_resp_clone
                            .send((MessageType::INIT, worker.id, task.id, Instant::now()))
                            .unwrap();

                        // Process the task
                        worker.process_task(task.clone());

                        // Send completion message
                        sender_resp_clone
                            .send((MessageType::FINAL, worker.id, task.id, Instant::now()))
                            .unwrap();
                    }
                    Err(_) => {
                        // The channel is closed, so break out of the loop
                        drop(_reciever);
                        break;
                    }
                }
            }
        });
        worker_handles.push(handle);
    }

    // Send tasks to the worker threads
    for task in tasks {
        let _ = sender.send(task);
    }

    // Define the timeout duration
    let timeout_duration = Duration::from_secs(3);
    let mut worker_task_map: HashMap<(u32, u32), Instant> = HashMap::new();
    let mut completed_count = 0;

    while completed_count < num_tasks {
        let (msg_type, worker_id, task_id, time) = receiver_resp.recv().unwrap();

        match msg_type {
            MessageType::INIT => {
                worker_task_map.insert((worker_id, task_id), time);
            }
            MessageType::FINAL => {
                // Calculate the elapsed time
                if let Some(start_time) = worker_task_map.remove(&(worker_id, task_id)) {
                    let elapsed = time.duration_since(start_time);
                    if elapsed > timeout_duration {
                        println!(
                            "Timeout for Task ID: {} | Worker ID: {} ",
                            task_id, worker_id
                        );
                    } else {
                        println!(
                            "Successfully processed Task ID: {} | Worker ID: {} ",
                            task_id, worker_id
                        );
                    }
                }
                completed_count += 1;
            }
        }
    }

    drop(sender);

    for handle in worker_handles {
        handle.join().unwrap();
    }
}
