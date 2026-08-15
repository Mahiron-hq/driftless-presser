//! Decides *when* the next click should fire, decoupled from the actual
//! input-simulation call so the timing math can be unit tested without a
//! Windows target. See [`windows_impl`] for the `SendInput` side.

use std::time::{Duration, Instant};

/// Drift-free periodic scheduler.
///
/// A naive `sleep(base_interval + jitter)` makes the average interval
/// slower than requested and drifts without bound. This keeps a fixed-step
/// timeline instead, so any interval that runs long is repaid by a shorter
/// one right after, and no click is ever skipped.
pub struct ClickScheduler {
    expected_next: Instant,
    base_interval: Duration,
    /// Backlog beyond this is forgiven rather than repaid as a burst of
    /// catch-up clicks, e.g. after the process is suspended for a while.
    max_backlog: Duration,
}

fn backlog_for(base_interval: Duration) -> Duration {
    (base_interval * 4).max(Duration::from_millis(500)) + Duration::from_secs(1)
}

impl ClickScheduler {
    pub fn new(now: Instant, base_interval: Duration) -> Self {
        ClickScheduler {
            expected_next: now,
            base_interval,
            max_backlog: backlog_for(base_interval),
        }
    }

    /// Changes the target frequency without resetting the timeline, so any
    /// accumulated lead/lag carries over into the new cadence.
    pub fn set_base_interval(&mut self, base_interval: Duration) {
        self.base_interval = base_interval;
        self.max_backlog = backlog_for(base_interval);
    }

    pub fn base_interval(&self) -> Duration {
        self.base_interval
    }

    /// Re-arms the schedule as if `now` were the last fire time.
    ///
    /// Needed when an external event interrupts a pending sleep: without
    /// this, the next `tick()` advances the grid a second time on top of
    /// an already-committed but unfired slot, turning an intended speed-up
    /// into a momentary slow-down.
    pub fn resync_to(&mut self, now: Instant) {
        self.expected_next = now;
    }

    /// Advances the schedule by one tick and returns how long to sleep
    /// (from `now`) before the next click.
    ///
    /// The grid advances unconditionally each tick rather than resetting to
    /// `now`, which is what lets a late interval be repaid by a shorter one
    /// afterward.
    pub fn tick(&mut self, now: Instant, jitter: Duration) -> Duration {
        self.expected_next += self.base_interval;

        if now > self.expected_next && now - self.expected_next > self.max_backlog {
            self.expected_next = now;
        }

        let target = self.expected_next + jitter;
        target.saturating_duration_since(now)
    }
}

/// Jitter range lives here once rather than scattered across call sites.
pub fn sample_jitter() -> Duration {
    Duration::from_millis(fastrand::u64(10..=20))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_interval_matches_configured_frequency_over_many_clicks() {
        let start = Instant::now();
        let base = Duration::from_millis(250); // 4 Hz
        let mut sched = ClickScheduler::new(start, base);

        let mut t = start;
        let mut last = start;
        let mut gaps = Vec::new();
        for _ in 0..5000 {
            let sleep = sched.tick(t, sample_jitter());
            t += sleep;
            gaps.push(t.duration_since(last));
            last = t;
        }

        let total: Duration = gaps.iter().sum();
        let avg_secs = total.as_secs_f64() / gaps.len() as f64;
        let expected_secs = base.as_secs_f64();
        assert!(
            (avg_secs - expected_secs).abs() < 0.0002,
            "average interval {avg_secs:.6}s should be ~{expected_secs:.6}s (4Hz)"
        );
    }

    #[test]
    fn average_interval_matches_configured_frequency_at_7hz() {
        let start = Instant::now();
        let base = Duration::from_secs_f64(1.0 / 7.0);
        let mut sched = ClickScheduler::new(start, base);

        let mut t = start;
        for _ in 0..5000 {
            let sleep = sched.tick(t, sample_jitter());
            t += sleep;
        }

        let total = t.duration_since(start);
        let avg_secs = total.as_secs_f64() / 5000.0;
        assert!(
            (avg_secs - base.as_secs_f64()).abs() < 0.0002,
            "average interval {avg_secs:.6}s should be ~{:.6}s (7Hz)",
            base.as_secs_f64()
        );
    }

    #[test]
    fn scheduling_latency_is_caught_up_not_absorbed_permanently() {
        let start = Instant::now();
        let base = Duration::from_millis(200);
        let mut sched = ClickScheduler::new(start, base);

        let sleep1 = sched.tick(start, Duration::ZERO);
        assert_eq!(sleep1, base);

        // Simulates the OS waking the thread 120ms later than requested.
        let late_now = start + sleep1 + Duration::from_millis(120);
        let sleep2 = sched.tick(late_now, Duration::ZERO);

        assert!(sleep2 < base, "must shrink to catch up, got {sleep2:?}");
        assert!(sleep2 <= Duration::from_millis(90), "got {sleep2:?}");
    }

    #[test]
    fn long_stall_resyncs_instead_of_bursting() {
        let start = Instant::now();
        let base = Duration::from_millis(250);
        let mut sched = ClickScheduler::new(start, base);

        // Simulates the whole process being suspended for 10 seconds.
        let now = start + Duration::from_secs(10);
        let sleep = sched.tick(now, Duration::ZERO);

        assert_eq!(sleep, Duration::ZERO, "expected an immediate resync, got {sleep:?}");

        let sleep2 = sched.tick(now, Duration::ZERO);
        assert_eq!(sleep2, base, "cadence should be normal again immediately after resync");
    }

    #[test]
    fn long_stall_resync_then_jittered_click_does_not_shift_the_grid() {
        let start = Instant::now();
        let base = Duration::from_millis(250);
        let mut sched = ClickScheduler::new(start, base);

        let now = start + Duration::from_secs(10);
        let jitter = Duration::from_millis(15);
        let sleep = sched.tick(now, jitter);
        assert_eq!(sleep, jitter, "resync target is `now`, so the sleep is exactly the jitter");

        let fire_time = now + sleep;
        let sleep2 = sched.tick(fire_time, Duration::ZERO);
        assert_eq!(sleep2, base - jitter);
    }

    #[test]
    fn switching_frequency_takes_effect_on_the_very_next_tick() {
        let start = Instant::now();
        let mut sched = ClickScheduler::new(start, Duration::from_millis(250));
        let _ = sched.tick(start, Duration::ZERO);

        sched.set_base_interval(Duration::from_millis(143));
        let now = start + Duration::from_millis(250);
        let sleep = sched.tick(now, Duration::ZERO);
        assert_eq!(sleep, Duration::from_millis(143));
    }

    #[test]
    fn sleep_duration_never_negative() {
        let start = Instant::now();
        let mut sched = ClickScheduler::new(start, Duration::from_millis(100));
        let now = start + Duration::from_secs(1);
        let sleep = sched.tick(now, Duration::ZERO);
        assert!(sleep >= Duration::ZERO);
    }

    #[test]
    fn jitter_is_always_within_bounds() {
        for _ in 0..10_000 {
            let j = sample_jitter();
            assert!(j >= Duration::from_millis(10) && j <= Duration::from_millis(20));
        }
    }

    #[test]
    fn interrupting_a_pending_sleep_without_resync_would_double_advance() {
        // Pins down the bug resync_to fixes: without it, an interrupt
        // handler that just loops back to tick() stacks a second interval
        // on the uncommitted slot, so accelerating would briefly do the
        // opposite.
        let start = Instant::now();
        let mut sched = ClickScheduler::new(start, Duration::from_millis(250));

        let sleep1 = sched.tick(start, Duration::ZERO);
        assert_eq!(sleep1, Duration::from_millis(250));

        let interrupt_at = start + Duration::from_millis(100);
        sched.set_base_interval(Duration::from_millis(143));
        let sleep2 = sched.tick(interrupt_at, Duration::ZERO);

        assert_eq!(sleep2, Duration::from_millis(293));
    }

    #[test]
    fn resync_to_makes_interrupted_frequency_change_instant() {
        let start = Instant::now();
        let mut sched = ClickScheduler::new(start, Duration::from_millis(250));

        let sleep1 = sched.tick(start, Duration::ZERO);
        assert_eq!(sleep1, Duration::from_millis(250));

        let interrupt_at = start + Duration::from_millis(100);
        sched.set_base_interval(Duration::from_millis(143));
        sched.resync_to(interrupt_at);

        let sleep2 = sched.tick(interrupt_at, Duration::ZERO);
        assert_eq!(sleep2, Duration::from_millis(143));
    }
}

#[cfg(windows)]
pub mod windows_impl {
    //! Runs the clicker thread: parked at ~0% CPU until activated, then
    //! drives `SendInput` calls via [`ClickScheduler`], switching cadence
    //! instantly on RMB changes.

    use super::{sample_jitter, ClickScheduler};
    use crate::config::Config;
    use crate::hook::{InputState, SYNTHETIC_MARKER};
    use std::sync::atomic::Ordering;
    use std::sync::mpsc::{Receiver, RecvTimeoutError};
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};
    use winapi::um::winuser::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_KEYBOARD, INPUT_u, KEYBDINPUT,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC,
    };

    /// Sent by the hook thread (and the GUI thread, for live edits) to the
    /// clicker thread.
    pub enum ClickerCommand {
        ToggleActive,
        /// RMB state changed; re-evaluate frequency immediately.
        Recheck,
        ConfigUpdated(Config),
        Shutdown,
    }

    /// Some engines only sample keyboard state once per frame; a same-
    /// instant down+up pair can land inside a single sample and never
    /// register as a press.
    const KEY_HOLD_DURATION: Duration = Duration::from_millis(30);

    /// Sends `vk` as a down+up pair using a scan code rather than a
    /// virtual-key code: matches what real hardware reports, which some
    /// engines (e.g. SDL2-based ones) require to recognize the press even
    /// though Windows itself accepts either. Tagged with `SYNTHETIC_MARKER`
    /// so the hook (see hook.rs) ignores its own output.
    fn send_key_press(vk: u16) {
        unsafe {
            let scan = MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) as u16;

            // Falls back to VK-driven input if this key has no scan-code
            // mapping in the active layout.
            let (event_vk, event_scan, scan_flag): (u16, u16, u32) = if scan != 0 {
                (0, scan, KEYEVENTF_SCANCODE)
            } else {
                (vk, 0, 0)
            };

            let mut down_union: INPUT_u = std::mem::zeroed();
            *down_union.ki_mut() = KEYBDINPUT {
                wVk: event_vk,
                wScan: event_scan,
                dwFlags: scan_flag,
                time: 0,
                dwExtraInfo: SYNTHETIC_MARKER,
            };
            let mut down = INPUT { type_: INPUT_KEYBOARD, u: down_union };
            SendInput(1, &mut down as *mut INPUT, std::mem::size_of::<INPUT>() as i32);

            thread::sleep(KEY_HOLD_DURATION);

            let mut up_union: INPUT_u = std::mem::zeroed();
            *up_union.ki_mut() = KEYBDINPUT {
                wVk: event_vk,
                wScan: event_scan,
                dwFlags: scan_flag | KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: SYNTHETIC_MARKER,
            };
            let mut up = INPUT { type_: INPUT_KEYBOARD, u: up_union };
            SendInput(1, &mut up as *mut INPUT, std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn interval_for(hz: f64) -> Duration {
        // A zero/non-finite Hz would reach from_secs_f64 and spin the CPU.
        let hz = if hz.is_finite() && hz > 0.01 { hz } else { 1.0 };
        Duration::from_secs_f64(1.0 / hz)
    }

    /// Blocks at 0% CPU while idle. The active loop uses `recv_timeout`
    /// instead of `thread::sleep` so a toggle-off or RMB change interrupts
    /// the wait immediately rather than waiting out the current interval.
    pub fn run(state: Arc<InputState>, rx: Receiver<ClickerCommand>, mut config: Config) {
        let mut active = false;
        let mut target_vk = crate::config::vk::from_name(&config.target_key).unwrap_or(b'E' as u16);
        let mut default_interval = interval_for(config.default_frequency_hz);
        let mut rmb_interval = interval_for(config.rmb_frequency_hz);
        let mut scheduler = ClickScheduler::new(Instant::now(), default_interval);

        loop {
            if !active {
                match rx.recv() {
                    Ok(ClickerCommand::ToggleActive) => {
                        active = true;
                        state.active.store(true, Ordering::Relaxed);
                        scheduler = ClickScheduler::new(Instant::now(), current_interval(&state, default_interval, rmb_interval));
                    }
                    Ok(ClickerCommand::ConfigUpdated(new_cfg)) => {
                        apply_config(&new_cfg, &mut config, &mut target_vk, &mut default_interval, &mut rmb_interval);
                    }
                    Ok(ClickerCommand::Recheck) => {}
                    Ok(ClickerCommand::Shutdown) | Err(_) => return,
                }
                continue;
            }

            let sleep_for = scheduler.tick(Instant::now(), sample_jitter());
            match rx.recv_timeout(sleep_for) {
                Ok(ClickerCommand::ToggleActive) => {
                    active = false;
                    state.active.store(false, Ordering::Relaxed);
                    continue;
                }
                Ok(ClickerCommand::Recheck) => {
                    // resync_to keeps this from stacking an extra interval
                    // on the slot that was committed but never fired.
                    let want = current_interval(&state, default_interval, rmb_interval);
                    if want != scheduler.base_interval() {
                        scheduler.set_base_interval(want);
                        scheduler.resync_to(Instant::now());
                    }
                    continue;
                }
                Ok(ClickerCommand::ConfigUpdated(new_cfg)) => {
                    apply_config(&new_cfg, &mut config, &mut target_vk, &mut default_interval, &mut rmb_interval);
                    scheduler.set_base_interval(current_interval(&state, default_interval, rmb_interval));
                    scheduler.resync_to(Instant::now());
                    continue;
                }
                Ok(ClickerCommand::Shutdown) => return,
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {
                    let want = current_interval(&state, default_interval, rmb_interval);
                    if want != scheduler.base_interval() {
                        scheduler.set_base_interval(want);
                    }
                    send_key_press(target_vk);
                }
            }
        }
    }

    fn current_interval(state: &InputState, default_interval: Duration, rmb_interval: Duration) -> Duration {
        if state.rmb_down.load(Ordering::Relaxed) {
            rmb_interval
        } else {
            default_interval
        }
    }

    fn apply_config(
        new_cfg: &Config,
        config: &mut Config,
        target_vk: &mut u16,
        default_interval: &mut Duration,
        rmb_interval: &mut Duration,
    ) {
        *config = new_cfg.clone();
        *target_vk = crate::config::vk::from_name(&config.target_key).unwrap_or(b'E' as u16);
        *default_interval = interval_for(config.default_frequency_hz);
        *rmb_interval = interval_for(config.rmb_frequency_hz);
    }
}
