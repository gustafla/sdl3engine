use std::{
    ffi::{c_char, CStr},
    ptr::{self, NonNull},
    sync::Mutex,
};

use anyhow::{Context, Result};

use sdl3_main::{app_event, app_init, app_iterate, app_quit, AppResult};

// You can `use sdl3_sys::everything::*` if you don't want to specify everything explicitly
use sdl3_sys::{
    error::SDL_GetError,
    events::{SDL_Event, SDL_EventType, SDL_EVENT_KEY_DOWN, SDL_EVENT_QUIT},
    gpu::{
        SDL_AcquireGPUCommandBuffer, SDL_BeginGPURenderPass, SDL_BindGPUGraphicsPipeline,
        SDL_CancelGPUCommandBuffer, SDL_ClaimWindowForGPUDevice, SDL_CreateGPUDevice,
        SDL_CreateGPUGraphicsPipeline, SDL_CreateGPUShader, SDL_DestroyGPUDevice,
        SDL_DrawGPUPrimitives, SDL_EndGPURenderPass, SDL_GPUColorTargetBlendState,
        SDL_GPUColorTargetDescription, SDL_GPUColorTargetInfo, SDL_GPUDepthStencilState,
        SDL_GPUDevice, SDL_GPUGraphicsPipeline, SDL_GPUGraphicsPipelineCreateInfo,
        SDL_GPUGraphicsPipelineTargetInfo, SDL_GPUMultisampleState, SDL_GPURasterizerState,
        SDL_GPUSampleCount, SDL_GPUShaderCreateInfo, SDL_GPUTexture, SDL_GPUVertexInputState,
        SDL_GetGPUSwapchainTextureFormat, SDL_ReleaseGPUGraphicsPipeline, SDL_ReleaseGPUShader,
        SDL_SubmitGPUCommandBuffer, SDL_WaitAndAcquireGPUSwapchainTexture,
        SDL_GPU_COMPAREOP_GREATER, SDL_GPU_CULLMODE_NONE, SDL_GPU_FILLMODE_FILL,
        SDL_GPU_FRONTFACE_COUNTER_CLOCKWISE, SDL_GPU_LOADOP_CLEAR,
        SDL_GPU_PRIMITIVETYPE_TRIANGLESTRIP, SDL_GPU_SHADERFORMAT_SPIRV,
        SDL_GPU_SHADERSTAGE_FRAGMENT, SDL_GPU_SHADERSTAGE_VERTEX, SDL_GPU_STOREOP_STORE,
    },
    init::{
        SDL_Init, SDL_SetAppMetadata, SDL_SetAppMetadataProperty, SDL_INIT_VIDEO,
        SDL_PROP_APP_METADATA_COPYRIGHT_STRING, SDL_PROP_APP_METADATA_CREATOR_STRING,
        SDL_PROP_APP_METADATA_TYPE_STRING, SDL_PROP_APP_METADATA_URL_STRING,
    },
    pixels::SDL_FColor,
    scancode::{SDL_Scancode, SDL_SCANCODE_ESCAPE, SDL_SCANCODE_Q},
    video::{SDL_CreateWindow, SDL_DestroyWindow, SDL_Window},
};

const SDL_WINDOW_WIDTH: i32 = 1280;
const SDL_WINDOW_HEIGHT: i32 = 720;

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()) }
        .to_string_lossy()
        .into_owned()
}

struct AppState {
    renderer: Renderer,
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

struct Renderer {
    window: NonNull<SDL_Window>,
    device: NonNull<SDL_GPUDevice>,
    pipeline: NonNull<SDL_GPUGraphicsPipeline>,
}

impl Renderer {
    fn new(mut device: NonNull<SDL_GPUDevice>, mut window: NonNull<SDL_Window>) -> Result<Self> {
        let pipeline = unsafe {
            let device = device.as_mut();
            let window = window.as_mut();
            let mut shadercreateinfo = SDL_GPUShaderCreateInfo {
                entrypoint: c"main".as_ptr(),
                format: SDL_GPU_SHADERFORMAT_SPIRV,
                stage: SDL_GPU_SHADERSTAGE_VERTEX,
                ..Default::default()
            };

            let vert = include_bytes!(concat!(env!("OUT_DIR"), "/quad.vert.spv"));
            shadercreateinfo.code_size = vert.len();
            shadercreateinfo.code = vert.as_ptr();
            let mut vertex_shader = NonNull::new(SDL_CreateGPUShader(device, &shadercreateinfo))
                .with_context(sdl_error)
                .context("Can't create vertex shader")?;

            let frag = include_bytes!(concat!(env!("OUT_DIR"), "/test.frag.spv"));
            shadercreateinfo.code_size = frag.len();
            shadercreateinfo.code = frag.as_ptr();
            shadercreateinfo.stage = SDL_GPU_SHADERSTAGE_FRAGMENT;
            let mut fragment_shader = NonNull::new(SDL_CreateGPUShader(device, &shadercreateinfo))
                .with_context(sdl_error)
                .context("Can't create fragment shader")?;

            let window_format = SDL_GetGPUSwapchainTextureFormat(device, window);
            let createinfo = SDL_GPUGraphicsPipelineCreateInfo {
                vertex_shader: vertex_shader.as_ptr(),
                fragment_shader: fragment_shader.as_ptr(),
                vertex_input_state: SDL_GPUVertexInputState {
                    num_vertex_buffers: 0,
                    num_vertex_attributes: 0,
                    ..Default::default()
                },
                primitive_type: SDL_GPU_PRIMITIVETYPE_TRIANGLESTRIP,
                rasterizer_state: SDL_GPURasterizerState {
                    fill_mode: SDL_GPU_FILLMODE_FILL,
                    cull_mode: SDL_GPU_CULLMODE_NONE,
                    front_face: SDL_GPU_FRONTFACE_COUNTER_CLOCKWISE,
                    ..Default::default()
                },
                multisample_state: SDL_GPUMultisampleState {
                    sample_count: SDL_GPUSampleCount::_1,
                    enable_mask: false,
                    ..Default::default()
                },
                depth_stencil_state: SDL_GPUDepthStencilState {
                    compare_op: SDL_GPU_COMPAREOP_GREATER,
                    enable_depth_test: false,
                    enable_stencil_test: false,
                    ..Default::default()
                },
                target_info: SDL_GPUGraphicsPipelineTargetInfo {
                    color_target_descriptions: &SDL_GPUColorTargetDescription {
                        format: window_format,
                        blend_state: SDL_GPUColorTargetBlendState::default(),
                    },
                    num_color_targets: 1,
                    has_depth_stencil_target: false,
                    ..Default::default()
                },
                props: 0,
            };
            let pipeline = NonNull::new(SDL_CreateGPUGraphicsPipeline(device, &createinfo))
                .with_context(sdl_error)
                .context("Can't create graphics pipeline")?;

            SDL_ReleaseGPUShader(device, fragment_shader.as_mut());
            SDL_ReleaseGPUShader(device, vertex_shader.as_mut());

            pipeline
        };

        Ok(Self {
            window,
            device,
            pipeline,
        })
    }

    fn render(&mut self) -> Result<()> {
        unsafe {
            let device = self.device.as_mut();
            let window = self.window.as_mut();

            let mut cmdbuf = NonNull::new(SDL_AcquireGPUCommandBuffer(device))
                .with_context(sdl_error)
                .context("Couldn't acquire command buffer")?;

            let mut swapchain_texture: *mut SDL_GPUTexture = ptr::null_mut();
            SDL_WaitAndAcquireGPUSwapchainTexture(
                cmdbuf.as_mut(),
                window,
                &mut swapchain_texture,
                ptr::null_mut(),
                ptr::null_mut(),
            )
            .then_some(())
            .with_context(sdl_error)
            .context("Swapchain texture acquisition failed")?;
            if swapchain_texture.is_null() {
                SDL_CancelGPUCommandBuffer(cmdbuf.as_mut())
                    .then_some(())
                    .with_context(sdl_error)
                    .context("No swapchain texture available")
                    .context("Cancelling acquired command buffer failed")?;
                return Ok(());
            }

            let color_target_info = SDL_GPUColorTargetInfo {
                texture: swapchain_texture,
                mip_level: 0,
                layer_or_depth_plane: 0,
                clear_color: SDL_FColor {
                    r: 0.,
                    g: 0.,
                    b: 0.,
                    a: 1.,
                },
                load_op: SDL_GPU_LOADOP_CLEAR,
                store_op: SDL_GPU_STOREOP_STORE,
                ..Default::default()
            };
            let render_pass =
                SDL_BeginGPURenderPass(cmdbuf.as_mut(), &color_target_info, 1, ptr::null());

            SDL_BindGPUGraphicsPipeline(render_pass, self.pipeline.as_mut());
            SDL_DrawGPUPrimitives(render_pass, 4, 1, 0, 0);

            SDL_EndGPURenderPass(render_pass);

            SDL_SubmitGPUCommandBuffer(cmdbuf.as_mut())
                .then_some(())
                .with_context(sdl_error)
                .context("Command buffer submission failed")?;

            Ok(())
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let device = self.device.as_mut();
            SDL_ReleaseGPUGraphicsPipeline(device, self.pipeline.as_mut());
            SDL_DestroyGPUDevice(device);
            SDL_DestroyWindow(self.window.as_mut());
        }
    }
}

#[app_iterate]
fn app_iterate(app: &mut AppState) -> AppResult {
    if let Err(e) = app.renderer.render() {
        eprintln!("{e:?}");
        AppResult::Failure
    } else {
        AppResult::Continue
    }
}

fn init() -> Result<AppState> {
    unsafe {
        SDL_SetAppMetadata(
            c"Rendering Engine".as_ptr(),
            c"1.0".as_ptr(),
            c"tech.mehu.engine".as_ptr(),
        )
        .then_some(())
        .with_context(sdl_error)
        .context("Could not set metadata")?;

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
            SDL_SetAppMetadataProperty(key, value)
                .then_some(())
                .with_context(sdl_error)
                .context("Could not set extended metadata")?;
        }

        SDL_Init(SDL_INIT_VIDEO)
            .then_some(())
            .with_context(sdl_error)
            .context("SDL_Init failed")?;

        let mut window = NonNull::new(SDL_CreateWindow(
            c"Mehu Demo".as_ptr(),
            SDL_WINDOW_WIDTH,
            SDL_WINDOW_HEIGHT,
            0,
        ))
        .with_context(sdl_error)
        .context("Can't create window")?;

        let mut device = NonNull::new(SDL_CreateGPUDevice(
            SDL_GPU_SHADERFORMAT_SPIRV,
            cfg!(debug_assertions),
            ptr::null(),
        ))
        .with_context(sdl_error)
        .context("Can't create GPU device")?;

        SDL_ClaimWindowForGPUDevice(device.as_mut(), window.as_mut())
            .then_some(())
            .with_context(sdl_error)
            .context("Failed to claim window for GPU device")?;

        let renderer = Renderer::new(device, window)?;
        Ok(AppState { renderer })
    }
}

#[app_init]
fn app_init() -> Option<Box<Mutex<AppState>>> {
    match init() {
        Ok(app) => Some(Box::new(Mutex::new(app))),
        Err(e) => {
            eprintln!("{e:?}");
            None
        }
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
