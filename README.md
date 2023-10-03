# chatterverse FakeYou Rust Library

Chatterverse FakeYou is a Rust library that provides a client for interacting with the FakeYou API. This library allows you to perform text-to-speech (TTS) inference and monitor TTS job status.

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [Examples](#examples)
- [Contributing](#contributing)
- [License](#license)

## Installation

To use the Chatterverse FakeYou Rust library in your project, add it as a dependency in your `Cargo.toml` file:

```toml
[dependencies]
chatterverse_fakeyou = {git = "https://github.com/chatterverse-ai/fakeyou-client.git"}
```

## Usage

```rust
extern crate chatterverse_fakeyou;

use chatterverse_fakeyou::Client;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the FakeYou client with your username and password
    let username = "your_username";
    let password = "your_password";
    let fakeyou_client = Client::init(username, password).await?;

    // Perform TTS inference
    let tts_model_token = "your_model_token";
    let inference_text = "Hello, world!";
    let tts_inference_response = fakeyou_client.tts_inference(tts_model_token, inference_text).await?;

    // Poll for TTS job status
    let inference_job_token = tts_inference_response.inference_job_token.expect("No job token found");
    let tts_job_response = fakeyou_client.poll_tts_job(inference_job_token).await?;

    println!("{:#?}", tts_job_response);
    
    Ok(())
}
```

## Contributing

Contributions are welcome! If you find a bug or have an enhancement in mind, please open an issue or submit a pull request on the [GitHub repository](https://github.com/chatterverse-ai/fakeyou-client).

## License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.