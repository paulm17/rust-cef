#[allow(unexpected_cfgs)]
#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::CString;

    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use rust_cef_startup::StartupSnapshot;
    use winit::dpi::PhysicalSize;
    use winit::window::Window;

    use crate::StartupUiConfig;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSRect {
        origin: NSPoint,
        size: NSSize,
    }

    const NS_BACKING_STORE_BUFFERED: usize = 2;
    const NS_WINDOW_ABOVE: isize = 1;

    pub struct SplashScreen {
        parent_window: *mut Object,
        overlay_window: *mut Object,
        title_label: *mut Object,
        subtitle_label: *mut Object,
        percent_label: *mut Object,
        status_label: *mut Object,
        track_view: *mut Object,
        fill_view: *mut Object,
    }

    impl SplashScreen {
        pub fn new(window: &Window, config: &StartupUiConfig) -> Result<Self, String> {
            let handle = window
                .window_handle()
                .map_err(|err| format!("failed to get macOS window handle: {err}"))?;
            let parent_view = match handle.as_raw() {
                RawWindowHandle::AppKit(appkit) => appkit.ns_view.as_ptr() as *mut Object,
                _ => return Err("expected AppKit window handle on macOS".to_string()),
            };

            unsafe {
                let parent_window: *mut Object = msg_send![parent_view, window];
                if parent_window.is_null() {
                    return Err("failed to resolve parent NSWindow for splash".to_string());
                }

                let parent_frame: NSRect = msg_send![parent_window, frame];
                let overlay_window = create_overlay_window(parent_frame, config);
                let content_view: *mut Object = msg_send![overlay_window, contentView];

                let title_label = create_label(config.title.as_str(), 30.0, true);
                let subtitle_label =
                    create_label(config.subtitle.as_deref().unwrap_or(""), 15.0, false);
                let percent_label = create_label("0%", 14.0, false);
                let status_label = create_label("Starting application", 15.0, false);
                let track_view = create_bar_segment((41, 49, 61), 255);
                let fill_view = create_bar_segment((109, 180, 255), 255);

                add_subview(content_view, title_label);
                add_subview(content_view, subtitle_label);
                add_subview(content_view, percent_label);
                add_subview(content_view, status_label);
                add_subview(content_view, track_view);
                add_subview(track_view, fill_view);

                let _: () = msg_send![parent_window, addChildWindow: overlay_window ordered: NS_WINDOW_ABOVE];
                let _: () = msg_send![overlay_window, orderFrontRegardless];

                let mut splash = Self {
                    parent_window,
                    overlay_window,
                    title_label,
                    subtitle_label,
                    percent_label,
                    status_label,
                    track_view,
                    fill_view,
                };
                splash.layout(window.inner_size());
                Ok(splash)
            }
        }

        pub fn update(
            &mut self,
            snapshot: &StartupSnapshot,
            ready_for_cef: bool,
            config: &StartupUiConfig,
        ) -> Result<(), String> {
            unsafe {
                let percent = if ready_for_cef {
                    "100%".to_string()
                } else {
                    format!("{}%", snapshot.aggregate_progress)
                };
                let status = if ready_for_cef {
                    "Loading complete".to_string()
                } else if config.show_milestone_label {
                    snapshot.status_text.clone()
                } else {
                    "Starting application".to_string()
                };

                set_label_text(self.title_label, config.title.as_str());
                set_label_text(
                    self.subtitle_label,
                    config.subtitle.as_deref().unwrap_or(""),
                );
                set_label_text(self.percent_label, percent.as_str());
                set_label_text(self.status_label, status.as_str());

                let progress = if ready_for_cef {
                    1.0
                } else {
                    f64::from(snapshot.aggregate_progress) / 100.0
                };
                update_fill_width(self.track_view, self.fill_view, progress);
            }

            Ok(())
        }

        pub fn resize(
            &mut self,
            size: PhysicalSize<u32>,
            snapshot: Option<&StartupSnapshot>,
            ready_for_cef: bool,
            config: &StartupUiConfig,
        ) -> Result<(), String> {
            self.layout(size);
            if let Some(snapshot) = snapshot {
                self.update(snapshot, ready_for_cef, config)?;
            }
            Ok(())
        }

        pub fn teardown(&mut self) {
            unsafe {
                let _: () = msg_send![self.parent_window, removeChildWindow: self.overlay_window];
                let _: () = msg_send![self.overlay_window, orderOut: std::ptr::null_mut::<Object>()];
                let _: () = msg_send![self.overlay_window, close];
            }
        }

        fn layout(&mut self, size: PhysicalSize<u32>) {
            if size.width == 0 || size.height == 0 {
                return;
            }

            unsafe {
                let parent_frame: NSRect = msg_send![self.parent_window, frame];
                let _: () = msg_send![self.overlay_window, setFrame: parent_frame display: 1u8];

                let width = size.width as f64;
                let height = size.height as f64;
                let title_y = height * 0.70;
                let subtitle_y = title_y - 38.0;
                let bar_width = (width - 96.0).max(220.0);
                let bar_x = ((width - bar_width) / 2.0).max(32.0);
                let bar_y = height * 0.54;
                let status_y = bar_y - 56.0;
                let percent_y = bar_y + 22.0;

                set_frame(
                    self.title_label,
                    NSRect {
                        origin: NSPoint {
                            x: 0.0,
                            y: title_y,
                        },
                        size: NSSize {
                            width,
                            height: 36.0,
                        },
                    },
                );
                set_frame(
                    self.subtitle_label,
                    NSRect {
                        origin: NSPoint {
                            x: 0.0,
                            y: subtitle_y,
                        },
                        size: NSSize {
                            width,
                            height: 20.0,
                        },
                    },
                );
                set_frame(
                    self.track_view,
                    NSRect {
                        origin: NSPoint { x: bar_x, y: bar_y },
                        size: NSSize {
                            width: bar_width,
                            height: 12.0,
                        },
                    },
                );
                set_frame(
                    self.status_label,
                    NSRect {
                        origin: NSPoint {
                            x: 0.0,
                            y: status_y,
                        },
                        size: NSSize {
                            width,
                            height: 20.0,
                        },
                    },
                );
                set_frame(
                    self.percent_label,
                    NSRect {
                        origin: NSPoint {
                            x: 0.0,
                            y: percent_y,
                        },
                        size: NSSize {
                            width,
                            height: 18.0,
                        },
                    },
                );

                update_fill_width(self.track_view, self.fill_view, 0.0);
            }
        }
    }

    unsafe fn create_overlay_window(frame: NSRect, config: &StartupUiConfig) -> *mut Object {
        let window: *mut Object = msg_send![class!(NSWindow), alloc];
        let window: *mut Object = msg_send![
            window,
            initWithContentRect: frame
            styleMask: 0usize
            backing: NS_BACKING_STORE_BUFFERED
            defer: 0u8
        ];

        let background = ns_color(config.colors.background);
        let _: () = msg_send![window, setOpaque: 1u8];
        let _: () = msg_send![window, setBackgroundColor: background];
        let _: () = msg_send![window, setHasShadow: 0u8];
        let _: () = msg_send![window, setIgnoresMouseEvents: 1u8];
        let _: () = msg_send![window, setReleasedWhenClosed: 0u8];

        let content_view: *mut Object = msg_send![window, contentView];
        let _: () = msg_send![content_view, setWantsLayer: 1u8];
        let layer: *mut Object = msg_send![content_view, layer];
        let cg_color: *mut Object = msg_send![background, CGColor];
        let _: () = msg_send![layer, setBackgroundColor: cg_color];

        window
    }

    unsafe fn create_label(text: &str, font_size: f64, prominent: bool) -> *mut Object {
        let field: *mut Object = msg_send![class!(NSTextField), alloc];
        let field: *mut Object = msg_send![
            field,
            initWithFrame: NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: NSSize {
                    width: 100.0,
                    height: 20.0,
                },
            }
        ];

        let _: () = msg_send![field, setEditable: 0u8];
        let _: () = msg_send![field, setSelectable: 0u8];
        let _: () = msg_send![field, setBordered: 0u8];
        let _: () = msg_send![field, setBezeled: 0u8];
        let _: () = msg_send![field, setDrawsBackground: 0u8];
        let _: () = msg_send![field, setAlignment: 1usize];
        let _: () = msg_send![field, setStringValue: ns_string(text)];
        let _: () = msg_send![field, setTextColor: ns_color(if prominent {
            (0.96, 0.97, 0.98, 1.0)
        } else {
            (0.70, 0.73, 0.78, 1.0)
        })];
        let font: *mut Object = msg_send![class!(NSFont), systemFontOfSize: font_size];
        let _: () = msg_send![field, setFont: font];

        field
    }

    unsafe fn create_bar_segment(rgb: (u8, u8, u8), alpha: u8) -> *mut Object {
        let view: *mut Object = msg_send![class!(NSView), alloc];
        let view: *mut Object = msg_send![
            view,
            initWithFrame: NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: NSSize {
                    width: 100.0,
                    height: 12.0,
                },
            }
        ];
        let _: () = msg_send![view, setWantsLayer: 1u8];
        let layer: *mut Object = msg_send![view, layer];
        let color = ns_color((
            f64::from(rgb.0) / 255.0,
            f64::from(rgb.1) / 255.0,
            f64::from(rgb.2) / 255.0,
            f64::from(alpha) / 255.0,
        ));
        let cg_color: *mut Object = msg_send![color, CGColor];
        let _: () = msg_send![layer, setBackgroundColor: cg_color];
        view
    }

    unsafe fn add_subview(parent: *mut Object, child: *mut Object) {
        let _: () = msg_send![parent, addSubview: child];
    }

    unsafe fn set_frame(view: *mut Object, frame: NSRect) {
        let _: () = msg_send![view, setFrame: frame];
    }

    unsafe fn update_fill_width(track_view: *mut Object, fill_view: *mut Object, progress: f64) {
        let track_bounds: NSRect = msg_send![track_view, bounds];
        let width = (track_bounds.size.width * progress.clamp(0.0, 1.0)).max(0.0);
        let fill_frame = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: NSSize {
                width,
                height: track_bounds.size.height,
            },
        };
        let _: () = msg_send![fill_view, setFrame: fill_frame];
    }

    unsafe fn set_label_text(label: *mut Object, text: &str) {
        let _: () = msg_send![label, setStringValue: ns_string(text)];
    }

    unsafe fn ns_string(value: &str) -> *mut Object {
        let c_value = CString::new(value).unwrap_or_default();
        let string: *mut Object = msg_send![class!(NSString), alloc];
        msg_send![string, initWithUTF8String: c_value.as_ptr()]
    }

    unsafe fn ns_color(color: (f64, f64, f64, f64)) -> *mut Object {
        msg_send![
            class!(NSColor),
            colorWithSRGBRed: color.0
            green: color.1
            blue: color.2
            alpha: color.3
        ]
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::fs;
    use std::path::PathBuf;

    use ab_glyph::{point, Font, FontArc, ScaleFont};
    use rust_cef_startup::StartupSnapshot;
    use tiny_skia::{Color, Paint, Pixmap, Rect, Transform};
    use wgpu::util::TextureBlitter;
    use winit::dpi::PhysicalSize;
    use winit::window::Window;

    use crate::StartupUiConfig;

    pub struct SplashScreen {
        renderer: WgpuSplashRenderer,
        font: Option<FontArc>,
    }

    impl SplashScreen {
        pub fn new(window: &Window, _config: &StartupUiConfig) -> Result<Self, String> {
            Ok(Self {
                renderer: WgpuSplashRenderer::new(window)?,
                font: load_font(),
            })
        }

        pub fn update(
            &mut self,
            snapshot: &StartupSnapshot,
            ready_for_cef: bool,
            config: &StartupUiConfig,
        ) -> Result<(), String> {
            self.renderer
                .render(snapshot, ready_for_cef, config, self.font.as_ref())
        }

        pub fn resize(
            &mut self,
            size: PhysicalSize<u32>,
            snapshot: Option<&StartupSnapshot>,
            ready_for_cef: bool,
            config: &StartupUiConfig,
        ) -> Result<(), String> {
            self.renderer.resize(size)?;
            if let Some(snapshot) = snapshot {
                self.update(snapshot, ready_for_cef, config)?;
            }
            Ok(())
        }

        pub fn teardown(&mut self) {}
    }

    struct WgpuSplashRenderer {
        _instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_config: wgpu::SurfaceConfiguration,
        splash_texture: wgpu::Texture,
        splash_view: wgpu::TextureView,
        blitter: TextureBlitter,
        size: PhysicalSize<u32>,
    }

    impl WgpuSplashRenderer {
        fn new(window: &Window) -> Result<Self, String> {
            let size = window.inner_size();
            if size.width == 0 || size.height == 0 {
                return Err("window size must be non-zero for splash renderer".to_string());
            }

            let instance = wgpu::Instance::default();
            let surface = unsafe {
                instance
                    .create_surface_unsafe(
                        wgpu::SurfaceTargetUnsafe::from_window(window)
                            .map_err(|err| format!("failed to create surface target: {err}"))?,
                    )
                    .map_err(|err| format!("failed to create wgpu surface: {err}"))?
            };

            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }))
                .map_err(|err| format!("failed to request splash adapter: {err}"))?;

            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                    .map_err(|err| format!("failed to request splash device: {err}"))?;

            let surface_config = surface
                .get_default_config(&adapter, size.width, size.height)
                .ok_or_else(|| "surface is not compatible with splash adapter".to_string())?;
            surface.configure(&device, &surface_config);

            let (splash_texture, splash_view) = create_splash_texture(&device, size);
            let blitter = TextureBlitter::new(&device, surface_config.format);

            Ok(Self {
                _instance: instance,
                surface,
                device,
                queue,
                surface_config,
                splash_texture,
                splash_view,
                blitter,
                size,
            })
        }

        fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), String> {
            if size.width == 0 || size.height == 0 {
                return Ok(());
            }

            self.size = size;
            self.surface_config.width = size.width;
            self.surface_config.height = size.height;
            self.surface.configure(&self.device, &self.surface_config);
            let (texture, view) = create_splash_texture(&self.device, size);
            self.splash_texture = texture;
            self.splash_view = view;
            Ok(())
        }

        fn render(
            &mut self,
            snapshot: &StartupSnapshot,
            ready_for_cef: bool,
            config: &StartupUiConfig,
            font: Option<&FontArc>,
        ) -> Result<(), String> {
            if self.size.width == 0 || self.size.height == 0 {
                return Ok(());
            }

            let mut pixmap = Pixmap::new(self.size.width, self.size.height)
                .ok_or_else(|| "failed to allocate splash pixmap".to_string())?;
            draw_splash(&mut pixmap, snapshot, ready_for_cef, config, font);

            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.splash_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixmap.data(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.size.width * 4),
                    rows_per_image: Some(self.size.height),
                },
                wgpu::Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                },
            );

            let frame = match self.surface.get_current_texture() {
                Ok(frame) => frame,
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    self.surface.configure(&self.device, &self.surface_config);
                    self.surface
                        .get_current_texture()
                        .map_err(|err| format!("failed to reacquire splash frame: {err}"))?
                }
                Err(err) => return Err(format!("failed to acquire splash frame: {err}")),
            };

            let target_view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder =
                self.device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("rust-cef splash encoder"),
                    });
            self.blitter
                .copy(&self.device, &mut encoder, &self.splash_view, &target_view);
            self.queue.submit([encoder.finish()]);
            frame.present();
            Ok(())
        }
    }

    fn create_splash_texture(
        device: &wgpu::Device,
        size: PhysicalSize<u32>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rust-cef splash texture"),
            size: wgpu::Extent3d {
                width: size.width.max(1),
                height: size.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn draw_splash(
        pixmap: &mut Pixmap,
        snapshot: &StartupSnapshot,
        ready_for_cef: bool,
        config: &StartupUiConfig,
        font: Option<&FontArc>,
    ) {
        let background = to_color(config.colors.background);
        pixmap.fill(background);

        let width = pixmap.width() as f32;
        let height = pixmap.height() as f32;
        let title_y = height * 0.34;
        let subtitle_y = title_y + 44.0;
        let bar_width = (width - 96.0).max(220.0);
        let bar_x = ((width - bar_width) / 2.0).max(32.0);
        let bar_y = height * 0.54;
        let bar_height = 18.0;
        let status_y = bar_y + 52.0;
        let percent_y = bar_y - 28.0;

        let mut track = Paint::default();
        track.set_color_rgba8(41, 49, 61, 255);
        let track_rect = Rect::from_xywh(bar_x, bar_y, bar_width, bar_height).unwrap();
        pixmap.fill_rect(track_rect, &track, Transform::identity(), None);

        let progress = if ready_for_cef {
            1.0
        } else {
            f32::from(snapshot.aggregate_progress) / 100.0
        };
        let progress_width = (bar_width * progress).max(if progress > 0.0 { 6.0 } else { 0.0 });
        if progress_width > 0.0 {
            let mut fill = Paint::default();
            fill.set_color_rgba8(109, 180, 255, 255);
            let fill_rect = Rect::from_xywh(bar_x, bar_y, progress_width, bar_height).unwrap();
            pixmap.fill_rect(fill_rect, &fill, Transform::identity(), None);
        }

        let title = config.title.as_str();
        let subtitle = config.subtitle.as_deref().unwrap_or("");
        let status = if ready_for_cef {
            "Loading complete".to_string()
        } else if config.show_milestone_label {
            snapshot.status_text.clone()
        } else {
            "Starting application".to_string()
        };
        let percent = if ready_for_cef {
            "100%".to_string()
        } else {
            format!("{}%", snapshot.aggregate_progress)
        };

        if let Some(font) = font {
            draw_centered_text(
                pixmap,
                font,
                title,
                30.0,
                width * 0.5,
                title_y,
                config.colors.foreground,
            );
            if !subtitle.is_empty() {
                draw_centered_text(
                    pixmap,
                    font,
                    subtitle,
                    15.0,
                    width * 0.5,
                    subtitle_y,
                    config.colors.secondary_text,
                );
            }
            draw_centered_text(
                pixmap,
                font,
                status.as_str(),
                15.0,
                width * 0.5,
                status_y,
                config.colors.secondary_text,
            );
            draw_centered_text(
                pixmap,
                font,
                percent.as_str(),
                14.0,
                width * 0.5,
                percent_y,
                config.colors.secondary_text,
            );
        }
    }

    fn draw_centered_text(
        pixmap: &mut Pixmap,
        font: &FontArc,
        text: &str,
        px_size: f32,
        center_x: f32,
        baseline_y: f32,
        color: (f64, f64, f64, f64),
    ) {
        let scaled = font.as_scaled(px_size);
        let total_width = measure_text(&scaled, text);
        let mut pen_x = center_x - (total_width / 2.0);

        for ch in text.chars() {
            let glyph = scaled
                .glyph_id(ch)
                .with_scale_and_position(px_size, point(pen_x, baseline_y));
            if let Some(outlined) = font.outline_glyph(glyph.clone()) {
                let bounds = outlined.px_bounds();
                outlined.draw(|x, y, coverage| {
                    let px = bounds.min.x.floor() as i32 + x as i32;
                    let py = bounds.min.y.floor() as i32 + y as i32;
                    blend_pixel(pixmap, px, py, color, coverage);
                });
            }
            pen_x += scaled.h_advance(glyph.id);
        }
    }

    fn measure_text<F: Font>(font: &impl ScaleFont<F>, text: &str) -> f32 {
        text.chars()
            .map(|ch| font.h_advance(font.glyph_id(ch)))
            .sum::<f32>()
    }

    fn blend_pixel(
        pixmap: &mut Pixmap,
        x: i32,
        y: i32,
        color: (f64, f64, f64, f64),
        coverage: f32,
    ) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as u32;
        let y = y as u32;
        if x >= pixmap.width() || y >= pixmap.height() {
            return;
        }

        let alpha = (color.3 as f32 * coverage).clamp(0.0, 1.0);
        let idx = ((y * pixmap.width() + x) * 4) as usize;
        let pixels = pixmap.data_mut();

        let dst_r = pixels[idx] as f32 / 255.0;
        let dst_g = pixels[idx + 1] as f32 / 255.0;
        let dst_b = pixels[idx + 2] as f32 / 255.0;
        let dst_a = pixels[idx + 3] as f32 / 255.0;

        let src_r = color.0 as f32;
        let src_g = color.1 as f32;
        let src_b = color.2 as f32;

        let out_a = alpha + dst_a * (1.0 - alpha);
        let out_r = if out_a > 0.0 {
            (src_r * alpha + dst_r * dst_a * (1.0 - alpha)) / out_a
        } else {
            0.0
        };
        let out_g = if out_a > 0.0 {
            (src_g * alpha + dst_g * dst_a * (1.0 - alpha)) / out_a
        } else {
            0.0
        };
        let out_b = if out_a > 0.0 {
            (src_b * alpha + dst_b * dst_a * (1.0 - alpha)) / out_a
        } else {
            0.0
        };

        pixels[idx] = (out_r * 255.0).round().clamp(0.0, 255.0) as u8;
        pixels[idx + 1] = (out_g * 255.0).round().clamp(0.0, 255.0) as u8;
        pixels[idx + 2] = (out_b * 255.0).round().clamp(0.0, 255.0) as u8;
        pixels[idx + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    fn to_color(color: (f64, f64, f64, f64)) -> Color {
        Color::from_rgba(
            color.0.clamp(0.0, 1.0) as f32,
            color.1.clamp(0.0, 1.0) as f32,
            color.2.clamp(0.0, 1.0) as f32,
            color.3.clamp(0.0, 1.0) as f32,
        )
        .unwrap_or(Color::BLACK)
    }

    fn load_font() -> Option<FontArc> {
        font_candidates()
            .into_iter()
            .find_map(|path| fs::read(path).ok())
            .and_then(|bytes| FontArc::try_from_vec(bytes).ok())
    }

    fn font_candidates() -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        #[cfg(target_os = "windows")]
        {
            candidates.extend(
                ["C:\\Windows\\Fonts\\arial.ttf", "C:\\Windows\\Fonts\\segoeui.ttf"]
                    .into_iter()
                    .map(PathBuf::from),
            );
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            candidates.extend(
                [
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                    "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
                    "/usr/share/fonts/TTF/DejaVuSans.ttf",
                ]
                .into_iter()
                .map(PathBuf::from),
            );
        }

        candidates
    }
}

pub use imp::SplashScreen;
