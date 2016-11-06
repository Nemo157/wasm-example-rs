#![feature(const_fn)]
#![feature(drop_types_in_const)]

use std::cell::RefCell;
use std::ffi::CString;
use std::ptr;
use std::os::raw::{c_int, c_void, c_char, c_double, c_long, c_ushort};

thread_local!(static callback_holder: RefCell<Option<MouseCallbackHolder<'static>>> = RefCell::new(None));

fn main() {
    println!("Hello, Emscripten!");

    callback_holder.with(|holder| {
        *holder.borrow_mut() = Some(set_click_callback("#document", false, |ev| {
            println!("Clicked at {:?}", (ev.screenX, ev.screenY));
            false
        }).unwrap());
    });

    set_main_loop_callback(|| {
    });
}

type em_result = c_int;
type em_bool = c_int;

const EM_TRUE: em_bool = 1;
const EM_FALSE: em_bool = 0;

const EM_RESULT_SUCCESS: em_result = 0;
const EM_RESULT_DEFERRED: em_result = 1;
const EM_RESULT_NOT_SUPPORTED: em_result = -1;
const EM_RESULT_FAILED_NOT_DEFERRED: em_result = -2;
const EM_RESULT_INVALID_TARGET: em_result = -3;
const EM_RESULT_UNKNOWN_TARGET: em_result = -4;
const EM_RESULT_INVALID_PARAM: em_result = -5;
const EM_RESULT_FAILED: em_result = -6;
const EM_RESULT_NO_DATA: em_result = -7;
const EM_RESULT_TIMED_OUT: em_result = -8;

fn em_bool(b: bool) -> em_bool {
    if b { EM_TRUE } else { EM_FALSE }
}

#[repr(C)]
struct EmMouseEvent {
    timestamp: c_double,
    screenX: c_long,
    screenY: c_long,
    clientX: c_long,
    clientY: c_long,
    ctrlKey: em_bool,
    shiftKey: em_bool,
    altKey: em_bool,
    metaKey: em_bool,
    button: c_ushort,
    buttons: c_ushort,
    movementX: c_long,
    movementY: c_long,
    targetX: c_long,
    targetY: c_long,
    canvasX: c_long,
    canvasY: c_long,
    padding: c_long,
}

type em_mouse_callback_func = unsafe extern fn(event_type: c_int, mouse_event: *const EmMouseEvent, user_data: *mut c_void) -> em_bool;
type em_mouse_callback_register_func = unsafe extern fn(target: *const c_char, user_data: *mut c_void, use_capture: em_bool, callback: Option<em_mouse_callback_func>) -> em_result;
extern {
    fn emscripten_set_click_callback(target: *const c_char, user_data: *mut c_void, use_capture: em_bool, callback: Option<em_mouse_callback_func>) -> em_result;
}

struct MouseCallbackHolder<'a> {
    target: CString,
    callback: Box<Box<FnMut(&EmMouseEvent) -> bool + 'a>>,
    deregister: em_mouse_callback_register_func,
}

impl<'a> Drop for MouseCallbackHolder<'a> {
    fn drop(&mut self) {
        unsafe { (self.deregister)(self.target.as_ptr(), ptr::null_mut(), em_bool(false), None) };
    }
}

fn set_click_callback<'a, F: 'a>(target: &str, use_capture: bool, callback: F) -> Result<MouseCallbackHolder<'a>, em_result> where F: FnMut(&EmMouseEvent) -> bool + 'a {
    let target = CString::new(target).unwrap();
    let callback = Box::new(Box::new(callback) as Box<FnMut(&EmMouseEvent) -> bool>);
    let callback_ptr = &*callback as *const _ as *mut c_void;

    let result = unsafe {
        emscripten_set_click_callback(target.as_ptr(), callback_ptr, em_bool(use_capture), Some(c_click_callback))
    };

    unsafe extern "C" fn c_click_callback(event_type: c_int, mouse_event: *const EmMouseEvent, user_data: *mut c_void) -> em_bool {
        let callback = user_data as *mut Box<FnMut(&EmMouseEvent) -> bool>;
        em_bool((*callback)(&*mouse_event))
    }

    if result >= EM_RESULT_SUCCESS {
        Ok(MouseCallbackHolder {
            target: target,
            callback: callback,
            deregister: emscripten_set_click_callback,
        })
    } else {
        Err(result)
    }
}

#[allow(non_camel_case_types)]
type em_callback_func = unsafe extern fn();
extern {
    fn emscripten_set_main_loop(func: em_callback_func, fps: c_int, simulate_infinite_loop: c_int);
}

thread_local!(static MAIN_LOOP_CALLBACK: RefCell<*mut c_void> = RefCell::new(ptr::null_mut()));

fn set_main_loop_callback<F>(callback: F) -> ! where F: FnMut() {
    MAIN_LOOP_CALLBACK.with(|log| {
        *log.borrow_mut() = &callback as *const _ as *mut c_void;
    });

    unsafe { emscripten_set_main_loop(wrapper::<F>, 0, 1); }
    unreachable!();

    unsafe extern "C" fn wrapper<F>() where F : FnMut() {
        MAIN_LOOP_CALLBACK.with(|z| {
            let closure = *z.borrow_mut() as *mut F;
            (*closure)();
        });
    }
}
