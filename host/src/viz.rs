//! Real-time audio visualizer: WASAPI loopback capture -> FFT -> frequency bars (0..1).
use std::time::Duration;

use rustfft::{num_complex::Complex, FftPlanner};
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

pub const BARS: usize = 48;
const FFT_SIZE: usize = 1024;

/// Spawn the capture+FFT loop; `on_bars` is called with the smoothed bar levels.
pub fn run<F: FnMut(&[f32; BARS]) + Send + 'static>(mut on_bars: F) {
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator =
            match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                Ok(e) => e,
                Err(_) => return,
            };
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return,
        };
        let client: IAudioClient = match device.Activate(CLSCTX_ALL, None) {
            Ok(c) => c,
            Err(_) => return,
        };
        let mix = match client.GetMixFormat() {
            Ok(m) => m,
            Err(_) => return,
        };
        let channels = (*mix).nChannels as usize;
        if client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                10_000_000,
                0,
                mix,
                None,
            )
            .is_err()
        {
            return;
        }
        let capture: IAudioCaptureClient = match client.GetService() {
            Ok(c) => c,
            Err(_) => return,
        };
        if client.Start().is_err() {
            return;
        }

        let fft = FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        let mut samples: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
        let mut smooth = [0f32; BARS];
        let half = FFT_SIZE / 2;

        loop {
            std::thread::sleep(Duration::from_millis(16));
            let mut pkt = capture.GetNextPacketSize().unwrap_or(0);
            while pkt > 0 {
                let mut data: *mut u8 = std::ptr::null_mut();
                let mut frames = 0u32;
                let mut flags = 0u32;
                if capture
                    .GetBuffer(&mut data, &mut frames, &mut flags, None, None)
                    .is_err()
                {
                    break;
                }
                if !data.is_null() && frames > 0 {
                    let buf = std::slice::from_raw_parts(data as *const f32, frames as usize * channels);
                    for f in 0..frames as usize {
                        let mut s = 0f32;
                        for c in 0..channels {
                            s += buf[f * channels + c];
                        }
                        samples.push(s / channels as f32);
                    }
                }
                let _ = capture.ReleaseBuffer(frames);
                pkt = capture.GetNextPacketSize().unwrap_or(0);
            }

            while samples.len() >= FFT_SIZE {
                let mut spec: Vec<Complex<f32>> = samples[..FFT_SIZE]
                    .iter()
                    .enumerate()
                    .map(|(i, &s)| {
                        let w = 0.5
                            - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0)).cos();
                        Complex { re: s * w, im: 0.0 }
                    })
                    .collect();
                fft.process(&mut spec);

                let mut bars = [0f32; BARS];
                for (b, bar) in bars.iter_mut().enumerate() {
                    let lo = (half as f32).powf(b as f32 / BARS as f32) as usize;
                    let hi = ((half as f32).powf((b as f32 + 1.0) / BARS as f32) as usize)
                        .max(lo + 1)
                        .min(half);
                    let mut m = 0f32;
                    for k in lo..hi {
                        m += spec[k].norm();
                    }
                    *bar = (m / (hi - lo) as f32 * 0.02).sqrt().min(1.0);
                }
                for b in 0..BARS {
                    smooth[b] = smooth[b] * 0.55 + bars[b] * 0.45;
                }
                on_bars(&smooth);
                samples.drain(..FFT_SIZE / 2); // 50% overlap
            }
        }
    });
}
