use core::sync::atomic::{AtomicU32, Ordering};
use embassy_stm32::interrupt::typelevel::{Binding, Handler, Interrupt};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer;
use embassy_stm32::timer::input_capture::CapturePin;
use embassy_stm32::timer::low_level::{
    CountingMode, FilterValue, InputCaptureMode, InputTISelection,
};
use embassy_stm32::timer::{GeneralInstance32bit4Channel, TimerChannel};

pub struct Encoder<'d, T: GeneralInstance32bit4Channel> {
    tim: &'d timer::low_level::Timer<'d, T>,
    state: &'d EncoderState,
    max_period: u32,
}

impl<'d, T: GeneralInstance32bit4Channel> Encoder<'d, T> {
    fn new(
        tim: &'d timer::low_level::Timer<'d, T>,
        state: &'d EncoderState,
        max_period: u32,
    ) -> Self {
        Self {
            tim,
            state,
            max_period,
        }
    }

    /// Returns the period of the last tick in timer ticks, or None if the encoder is not moving (i.e. the period is greater than max_period).
    pub fn get_period(&self) -> Option<u32> {
        if self
            .tim
            .regs_gp32()
            .cnt()
            .read()
            .wrapping_sub(self.state.prev_count.load(Ordering::Relaxed))
            > self.max_period
        {
            None
        } else {
            Some(self.state.get_period())
        }
    }
}

#[derive(Default)]
struct EncoderState {
    prev_count: AtomicU32,
    tick_period: AtomicU32,
}

impl EncoderState {
    #[inline]
    fn update_period(&self, count: u32) {
        self.tick_period.store(
            // The wrapping_sub handles the case where the counter overflows (in the case where the period is less than 2^32 ticks)
            count.wrapping_sub(self.prev_count.load(Ordering::Relaxed)),
            Ordering::Relaxed,
        );
        self.prev_count.store(count, Ordering::Relaxed);
    }

    #[inline]
    fn get_period(&self) -> u32 {
        self.tick_period.load(Ordering::Relaxed)
    }
}

type EncoderStates = [EncoderState; 4];

pub struct Timer<T: GeneralInstance32bit4Channel> {
    tim: timer::low_level::Timer<'static, T>,
    encoder_states: EncoderStates,
}

impl<T: GeneralInstance32bit4Channel> Timer<T> {
    pub fn new<Irq, H>(mut tim: timer::low_level::Timer<'static, T>, _irq: Irq, freq: Hertz) -> Self
    where
        H: Handler<T::CaptureCompareInterrupt>,
        Irq: Binding<T::CaptureCompareInterrupt, H>,
    {
        tim.set_counting_mode(CountingMode::EdgeAlignedUp);
        tim.set_tick_freq(freq);

        Self {
            tim,
            encoder_states: Default::default(),
        }
    }

    pub fn start(&self) {
        // Prevent unwanted interrupts from being triggered when starting the timer
        self.tim.regs_gp32().sr().write(|w| w.0 = 0);
        T::CaptureCompareInterrupt::unpend();

        unsafe {
            T::CaptureCompareInterrupt::enable();
        }
        self.tim.start();
    }

    pub fn add_encoder<C: TimerChannel>(
        &self,
        _pin: CapturePin<'static, T, C>,
        filter_value: FilterValue,
        max_period: u32,
    ) -> Encoder<'_, T> {
        let channel = C::CHANNEL;

        self.tim
            .set_input_ti_selection(channel, InputTISelection::Normal);
        self.tim
            .set_input_capture_mode(channel, InputCaptureMode::Falling);
        self.tim.enable_input_interrupt(channel, true);
        self.tim.set_input_capture_filter(channel, filter_value);
        self.tim.enable_channel(channel, true);

        // This consumes the pin, but without dropping it. Forget it because the Encoder does not need it.
        core::mem::forget(_pin);

        Encoder::new(
            &self.tim,
            &self.encoder_states[channel as usize],
            max_period,
        )
    }

    pub fn on_interrupt(&self) {
        let sr = self.tim.regs_gp32().sr().read();

        for i in 0..4 {
            if sr.ccif(i) {
                // The reading of the ccr register clears the interrupt flag
                self.encoder_states[i].update_period(self.tim.regs_gp32().ccr(i).read());
            }
        }
    }
}
