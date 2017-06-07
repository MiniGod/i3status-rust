use std::time::Duration;
use std::process::Command;

use block::Block;
use widgets::button::ButtonWidget;
use widget::{I3BarWidget, State};
use input::{I3BarEvent, MouseButton};

use serde_json::Value;
use uuid::Uuid;

struct SoundDevice {
    name: String,
    volume: u32,
    muted: bool,
}

impl SoundDevice {
    fn new(name: &str) -> Self {
        let mut sd = SoundDevice {
            name: String::from(name),
            volume: 0,
            muted: false,
        };
        sd.get_info();
        sd
    }

    fn get_info(&mut self) -> bool {
        let output = Command::new("sh")
            .args(&["-c", format!("amixer get {}", self.name).as_str()])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());

        if let Ok(output) = output {
            if let Some(last_line) = (&output).lines()
                                              .into_iter()
                                              .last() {

                let last = last_line.split_whitespace()
                                    .into_iter()
                                    .filter(|x| x.starts_with('[') && !x.contains("dB"))
                                    .map(|s| s.trim_matches(FILTER))
                                    .collect::<Vec<&str>>();

                if let Some(vol_raw) = last.get(0) {
                    if let Ok(vol) = vol_raw.parse::<u32>() {
                        self.volume = vol;
                    }
                }

                if let Some(mute_raw) = last.get(1) {
                    self.muted = match *mute_raw {
                        "on" => false,
                        "off" => true,
                        _ => false
                    };
                    return true;
                }
            }
        }
        false
    }

    fn set_volume(&mut self, step: i32) {
       if Command::new("sh")
            .args(&["-c", format!("amixer set {} {}%",
                                  self.name,
                                  (self.volume as i32 + step) as u32).as_str()])
            .output().is_ok() {

            self.volume = (self.volume as i32 + step) as u32;
        }
    }

    fn toggle(&mut self) {
        if Command::new("sh")
              .args(&["-c", format!("amixer set {} toggle",
                                    self.name).as_str()])
              .output().is_ok() {

            self.muted = !self.muted;
        }
    }
}

// TODO: Use the alsa control bindings to implement push updates
// TODO: Allow for custom audio devices instead of Master
pub struct Sound {
    text: ButtonWidget,
    id: String,
    devices: Vec<SoundDevice>,
    update_interval: Duration,
    step_width: u32,
    current_idx: usize,
    theme: Value
}

impl Sound {
    pub fn new(config: Value, theme: Value) -> Sound {
        {
            let id = Uuid::new_v4().simple().to_string();
            let mut step_width = get_u64_default!(config, "step_width", 5) as u32;
            if step_width > 50 {
                step_width = 50;
            }
            let mut sound = Sound {
                text: ButtonWidget::new(theme.clone(), &id).with_icon("volume_empty"),
                id,
                devices: Vec::new(),
                update_interval: Duration::new(get_u64_default!(config, "interval", 2), 0),
                step_width: step_width,
                current_idx: 0,
                theme,
            };
            sound.devices.push(SoundDevice::new("Master"));
            sound
        }
    }

    fn display(&mut self) {
        if let Some(device) = self.devices.get_mut(self.current_idx) {
            if device.get_info() {
                if device.muted {
                    self.text.set_icon("volume_empty");
                    self.text.set_text(self.theme["icons"]["volume_muted"]
                             .as_str().expect("Wrong icon identifier!").to_owned());
                    self.text.set_state(State::Warning);
                } else {
                    self.text.set_icon(match device.volume {
                        0 ... 20 => "volume_empty",
                        20 ... 70 => "volume_half",
                        _ => "volume_full"
                    });
                    self.text.set_text(format!("{:02}%", device.volume));
                    self.text.set_state(State::Info);
                }
            } else {
                // TODO: Do proper error handling here instead of hiding in a corner
                self.text.set_icon("");
                self.text.set_text("".to_owned());
                self.text.set_state(State::Idle);
            }
        }
    }
}

// To filter [100%] output from amixer into 100
const FILTER: &[char] = &['[', ']', '%'];

impl Block for Sound
{
    fn update(&mut self) -> Option<Duration> {
        self.display();
        Some(self.update_interval.clone())
    }

    fn view(&self) -> Vec<&I3BarWidget> {
        vec![&self.text]
    }

    fn click(&mut self, e: &I3BarEvent) {
        if let Some(ref name) = e.name {
            if name.as_str() == self.id {
                if let Some(device) = self.devices.get_mut(self.current_idx) {
                    match e.button {
                        MouseButton::Right => {
                            device.toggle();
                        },
                        MouseButton::WheelUp => {
                            if device.volume <= (100 - self.step_width) {
                                device.set_volume(self.step_width as i32);
                            }
                        },
                        MouseButton::WheelDown => {
                            if device.volume >= self.step_width {
                                device.set_volume(- (self.step_width as i32));
                            }
                        },
                        _ => {}
                    }
                }
                self.display();
            }
        }
    }

    fn id(&self) -> &str {
        &self.id
    }
}