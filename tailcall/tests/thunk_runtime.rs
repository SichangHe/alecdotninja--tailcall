use std::mem::MaybeUninit;

use tailcall::{
    slot::{self, Slot},
    thunk::{Thunk, ThunkFn},
    trampoline::{self, Action},
};

#[test]
fn factorial_in_new_runtime() {
    assert!(factorial(5) == 120);
}

fn factorial(input: u64) -> u64 {
    #[inline(always)]
    fn call_factorial_inner<'slot>(
        slot: &'slot mut slot::Slot,
        accumulator: u64,
        input: u64,
    ) -> trampoline::Action<'slot, u64> {
        trampoline::call(slot, move |slot| {
            if input == 0 {
                return trampoline::done(slot, accumulator);
            }

            return call_factorial_inner(slot, accumulator * input, input - 1);
        })
    }

    fn factorial_inner(accumulator: u64, input: u64) -> u64 {
        trampoline::run(move |slot| call_factorial_inner(slot, accumulator, input))
    }

    factorial_inner(1, input)
}

#[test]
fn factorial_inlined_in_new_runtime() {
    assert!(factorial_inlined(5) == 120);
}

fn factorial_inlined(input: u64) -> u64 {
    fn call_factorial_inner<'slot>(
        slot: &'slot mut slot::Slot,
        accumulator: u64,
        input: u64,
    ) -> trampoline::Action<'slot, u64> {
        let fn_once = move |slot| {
            if input == 0 {
                return Action::Done(accumulator);
            }

            return call_factorial_inner(slot, accumulator * input, input - 1);
        };
        let slot_bytes: &mut MaybeUninit<_> = unsafe { &mut *slot.bytes.as_mut_ptr().cast() };
        let ptr = slot_bytes.write(fn_once);
        Action::Call(Thunk { ptr })
    }

    fn factorial_inner(accumulator: u64, input: u64) -> u64 {
        let slot = &mut Slot::new();

        let mut action = call_factorial_inner(slot, accumulator, input);

        loop {
            match action {
                Action::Done(value) => return value,
                Action::Call(thunk) => {
                    let ptr: *mut dyn ThunkFn<'_, _> = thunk.ptr;
                    core::mem::forget(thunk);

                    action = unsafe {
                        /* // Does not work because the type cannot be spelled out.
                        let in_slot: *mut _ = &mut *ptr;
                        let slot: &mut Slot = &mut *in_slot.cast();
                        let slot_bytes_mut: &mut MaybeUninit<_> =
                            &mut *slot.bytes.as_mut_ptr().cast();
                        let fn_once = slot_bytes_mut.assume_init_read();
                        fn_once(slot) */
                        (*ptr).call_once_in_slot()
                    };
                }
            }
        }
    }

    factorial_inner(1, input)
}
