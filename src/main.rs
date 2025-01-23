use core::ffi::c_char;
use std::{
    ptr::{self, NonNull},
    sync::Mutex,
};

use sdl3_main::{app_event, app_init, app_iterate, app_quit, AppResult};

// You can `use sdl3_sys::everything::*` if you don't want to specify everything explicitly
use sdl3_sys::{
    events::{SDL_Event, SDL_EventType, SDL_EVENT_KEY_DOWN, SDL_EVENT_QUIT},
    gpu::{
        SDL_ClaimWindowForGPUDevice, SDL_CreateGPUDevice, SDL_DestroyGPUDevice, SDL_GPUDevice,
        SDL_GPU_SHADERFORMAT_SPIRV,
    },
    init::{
        SDL_Init, SDL_SetAppMetadata, SDL_SetAppMetadataProperty, SDL_INIT_VIDEO,
        SDL_PROP_APP_METADATA_COPYRIGHT_STRING, SDL_PROP_APP_METADATA_CREATOR_STRING,
        SDL_PROP_APP_METADATA_TYPE_STRING, SDL_PROP_APP_METADATA_URL_STRING,
    },
    scancode::{SDL_Scancode, SDL_SCANCODE_ESCAPE, SDL_SCANCODE_Q},
    video::{SDL_CreateWindow, SDL_DestroyWindow, SDL_Window},
};

const SDL_WINDOW_WIDTH: i32 = 1280;
const SDL_WINDOW_HEIGHT: i32 = 720;

struct AppState {
    window: NonNull<SDL_Window>,
    gpu: NonNull<SDL_GPUDevice>,
    renderer: Renderer,
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            SDL_DestroyGPUDevice(self.gpu.as_mut());
            SDL_DestroyWindow(self.window.as_mut());
        }
    }
}

unsafe impl Send for AppState {}

impl AppState {
    fn handle_key_event(&mut self, key_code: SDL_Scancode) -> AppResult {
        match key_code {
            SDL_SCANCODE_ESCAPE | SDL_SCANCODE_Q => AppResult::Success,
            _ => AppResult::Continue,
        }
    }
}

struct Renderer {}

impl Renderer {
    fn new() -> Self {
        Self {}
    }

    fn render(&self, gpu: *mut SDL_GPUDevice) {}
}

#[app_iterate]
fn app_iterate(app: &mut AppState) -> AppResult {
    //let ctx = &mut app.ctx;
    unsafe {
        // let now = SDL_GetTicks();
        // SDL_SetRenderDrawColor(app.renderer, 0, 0, 0, SDL_ALPHA_OPAQUE);
        // SDL_RenderClear(app.renderer);

        // SDL_RenderPresent(app.renderer);
        AppResult::Continue
    }
}

#[app_init]
fn app_init() -> Option<Box<Mutex<AppState>>> {
    unsafe {
        if !SDL_SetAppMetadata(
            c"Rendering Engine".as_ptr(),
            c"1.0".as_ptr(),
            c"tech.mehu.engine".as_ptr(),
        ) {
            return None;
        }

        const EXTENDED_METADATA: &[(*const c_char, *const c_char)] = &[
            (
                SDL_PROP_APP_METADATA_URL_STRING,
                c"https://mehu.tech/".as_ptr(),
            ),
            (SDL_PROP_APP_METADATA_CREATOR_STRING, c"luutifa".as_ptr()),
            (SDL_PROP_APP_METADATA_COPYRIGHT_STRING, c"2025".as_ptr()),
            (SDL_PROP_APP_METADATA_TYPE_STRING, c"demo".as_ptr()),
        ];

        for (key, value) in EXTENDED_METADATA.iter().copied() {
            if !SDL_SetAppMetadataProperty(key, value) {
                return None;
            }
        }

        if !SDL_Init(SDL_INIT_VIDEO) {
            return None;
        }

        let mut window = NonNull::new(SDL_CreateWindow(
            c"Mehu Demo".as_ptr(),
            SDL_WINDOW_WIDTH,
            SDL_WINDOW_HEIGHT,
            0,
        ))?;

        let mut gpu = NonNull::new(SDL_CreateGPUDevice(
            SDL_GPU_SHADERFORMAT_SPIRV,
            cfg!(debug_assertions),
            ptr::null(),
        ))?;

        SDL_ClaimWindowForGPUDevice(gpu.as_mut(), window.as_mut());

        let renderer = Renderer::new();

        Some(Box::new(Mutex::new(AppState {
            window,
            gpu,
            renderer,
        })))
    }
}

#[app_event]
fn app_event(app: &mut AppState, event: &SDL_Event) -> AppResult {
    unsafe {
        match SDL_EventType(event.r#type) {
            SDL_EVENT_QUIT => AppResult::Success,
            SDL_EVENT_KEY_DOWN => app.handle_key_event(event.key.scancode),
            _ => AppResult::Continue,
        }
    }
}

#[app_quit]
fn app_quit() {}
