enum Command {
    SetMqttTimeour(u32),
}

struct CommandMessage {
    command: Command,
}

impl CommandMessage {
    fn new(command: Command) -> Self {
        Self { command }
    }
}
