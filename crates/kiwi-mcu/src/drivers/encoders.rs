use core::sync::atomic::{AtomicU32, Ordering};
use embassy_stm32::interrupt::typelevel::Interrupt;
use embassy_stm32::time::mhz;
use embassy_stm32::timer;
use embassy_stm32::timer::input_capture::CapturePin;
use embassy_stm32::timer::low_level::{CountingMode, InputCaptureMode, InputTISelection, Timer};
use embassy_stm32::timer::{GeneralInstance1Channel, GeneralInstance4Channel, TimerChannel};

pub struct Channel<T: GeneralInstance1Channel, C: TimerChannel> {
    pin: CapturePin<'static, T, C>,
    prev_count: AtomicU32,
    tick_period: AtomicU32,
}

type Channels<T> = (
    Option<Channel<T, timer::Ch1>>,
    Option<Channel<T, timer::Ch2>>,
    Option<Channel<T, timer::Ch3>>,
    Option<Channel<T, timer::Ch4>>,
);

pub trait TimerChannelSlot<T: GeneralInstance4Channel>: TimerChannel + Sized {
    fn insert_into(channels: &mut Channels<T>, channel: Channel<T, Self>);
    fn get_from(channels: &Channels<T>) -> Option<&Channel<T, Self>>;
}

macro_rules! impl_timer_channel_slot {
    ($ch:ident, $idx:tt) => {
        impl<T: GeneralInstance4Channel> TimerChannelSlot<T> for timer::$ch {
            fn insert_into(channels: &mut Channels<T>, channel: Channel<T, Self>) {
                channels.$idx = Some(channel);
            }

            fn get_from(channels: &Channels<T>) -> Option<&Channel<T, Self>> {
                channels.$idx.as_ref()
            }
        }
    };
}

impl_timer_channel_slot!(Ch1, 0);
impl_timer_channel_slot!(Ch2, 1);
impl_timer_channel_slot!(Ch3, 2);
impl_timer_channel_slot!(Ch4, 3);

pub struct OneChannelEncodersTimer<T: GeneralInstance4Channel> {
    tim: Timer<'static, T>,
    channels: Channels<T>,
}

impl<T: GeneralInstance4Channel> OneChannelEncodersTimer<T> {
    pub fn new(mut tim: Timer<'static, T>) -> Self {
        tim.set_counting_mode(CountingMode::EdgeAlignedUp);
        tim.set_tick_freq(mhz(1));
        unsafe {
            T::CaptureCompareInterrupt::enable();
        }
        tim.start();

        Self {
            tim,
            channels: (None, None, None, None),
        }
    }

    pub fn add_channel<C: TimerChannel + TimerChannelSlot<T>>(
        &mut self,
        pin: CapturePin<'static, T, C>,
        filter_value: u8,
    ) {
        let channel = C::CHANNEL;

        self.tim
            .set_input_ti_selection(channel, InputTISelection::Normal);
        self.tim
            .set_input_capture_mode(channel, InputCaptureMode::Falling);
        self.tim.enable_input_interrupt(channel, true);
        self.tim
            .set_input_capture_filter(channel, filter_value.into());
        self.tim.enable_channel(channel, true);
        C::insert_into(
            &mut self.channels,
            Channel {
                pin,
                prev_count: AtomicU32::new(0),
                tick_period: AtomicU32::new(0),
            },
        );
    }

    pub fn update_channel_period<C: TimerChannel + TimerChannelSlot<T>>(&self, count: u32) {
        let channel = match C::get_from(&self.channels) {
            Some(c) => c,
            None => defmt::panic!(
                "Timer {} channel {} must be configured before updating its period.",
                core::any::type_name::<T>().split("::").last().unwrap(),
                core::any::type_name::<C>().split("::").last().unwrap()
            ),
        };

        channel.tick_period.store(
            count.wrapping_sub(channel.prev_count.load(Ordering::Relaxed)),
            Ordering::Relaxed,
        );
        channel.prev_count.store(count, Ordering::Relaxed);
    }

    pub fn get_channel_period<C: TimerChannel + TimerChannelSlot<T>>(&self) -> Option<u32> {
        let channel = C::get_from(&self.channels)?;
        Some(channel.tick_period.load(Ordering::Relaxed))
    }
}
