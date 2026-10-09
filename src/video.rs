//! Affichage de la vidéo dans la fenêtre Slint.
//!
//! mpv dessine chaque image dans une texture OpenGL qui appartient à la fenêtre ; l'interface
//! l'affiche comme une image ordinaire (`video-frame`) et dessine ses commandes par-dessus.
//! Tout ce qui touche à OpenGL se passe ici, sur le thread de l'interface, dans le rappel de
//! rendu de Slint.

use std::ffi::{c_void, CStr};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use glow::HasContext;
use slint::ComponentHandle;

use crate::{mpv, AppWindow};

/// Contexte OpenGL obtenu par la fenêtre (version et carte graphique), posé au premier rendu.
/// Sert à la vérification « Affichage » du démarrage.
static GL_INFO: std::sync::OnceLock<String> = std::sync::OnceLock::new();

pub fn gl_info() -> Option<String> {
    GL_INFO.get().cloned()
}

/// Lecteur à afficher (posé par la lecture, lu au moment du rendu) et signal « rendu prêt ».
static CURRENT: Mutex<Option<(Arc<mpv::Mpv>, Option<tokio::sync::oneshot::Sender<()>>)>> = Mutex::new(None);

/// Désigne le lecteur à afficher (None à la fin de la lecture) et demande un rafraîchissement.
/// Le récepteur renvoyé se déclenche quand le rendu vidéo est prêt : mpv doit l'avoir avant
/// d'ouvrir un fichier, sinon il désactive la vidéo.
pub fn attach(ui: &slint::Weak<AppWindow>, player: Option<Arc<mpv::Mpv>>) -> tokio::sync::oneshot::Receiver<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    *CURRENT.lock().unwrap() = player.map(|p| (p, Some(tx)));
    let _ = ui.upgrade_in_event_loop(|u| u.window().request_redraw());
    rx
}

struct Target {
    tex: glow::Texture,
    fbo: glow::Framebuffer,
    w: u32,
    h: u32,
}

struct State {
    gl: glow::Context,
    /// Contexte de rendu mpv + identité du lecteur auquel il appartient.
    render: Option<(usize, mpv::Render)>,
    target: Option<Target>,
}

/// Branche le rendu vidéo sur la fenêtre. Échoue si Slint n'utilise pas OpenGL
/// (rendu logiciel, par exemple) : la lecture est alors impossible.
pub fn install(ui: &AppWindow) -> Result<(), slint::SetRenderingNotifierError> {
    let weak = ui.as_weak();
    let mut state: Option<State> = None;
    // Diagnostic : TURTLEFIN_DEBUG_FRAMES=1 signale chaque image affichée plus de 25 ms après la
    // précédente (à 60 Hz, une image arrive toutes les 16,7 ms : au-delà, au moins une est sautée).
    // À combiner avec SLINT_DEBUG_PERFORMANCE=refresh_full_speed,console pour un rendu continu.
    let debug_frames = std::env::var_os("TURTLEFIN_DEBUG_FRAMES").is_some();
    // Durée de dessin d'une image (avant -> après le rendu) : l'écart entre deux images compterait
    // aussi les moments de repos, où rien n'est redessiné.
    let mut started: Option<std::time::Instant> = None;
    let t_start = std::time::Instant::now();
    // TURTLEFIN_DEBUG_FPS=1 : nombre d'images dessinées par seconde (avec
    // SLINT_DEBUG_PERFORMANCE=refresh_full_speed pour un rendu continu : le maximum possible).
    let debug_fps = std::env::var_os("TURTLEFIN_DEBUG_FPS").is_some();
    // TURTLEFIN_DEBUG_GAPS=1 (avec SLINT_DEBUG_PERFORMANCE=refresh_full_speed) : signale chaque pause
    // de plus de 40 ms entre deux images, c'est-à-dire un à-coup (fil de l'interface occupé).
    let debug_gaps = std::env::var_os("TURTLEFIN_DEBUG_GAPS").is_some();
    let mut last_frame: Option<std::time::Instant> = None;
    let (mut fps_n, mut fps_t) = (0u32, std::time::Instant::now());
    ui.window().set_rendering_notifier(move |rs, api| {
        if debug_fps && matches!(rs, slint::RenderingState::AfterRendering) {
            fps_n += 1;
            if fps_t.elapsed().as_secs_f64() >= 1.0 {
                eprintln!("[{:.0} s] {fps_n} images/s", t_start.elapsed().as_secs_f64());
                fps_n = 0;
                fps_t = std::time::Instant::now();
            }
        }
        if debug_gaps && matches!(rs, slint::RenderingState::AfterRendering) {
            let now = std::time::Instant::now();
            if let Some(t) = last_frame {
                let ms = now.duration_since(t).as_secs_f64() * 1000.0;
                if ms > 40.0 {
                    eprintln!("[{:.2} s] à-coup : {ms:.0} ms sans image", t_start.elapsed().as_secs_f64());
                }
            }
            last_frame = Some(now);
        }
        if debug_frames {
            match rs {
                slint::RenderingState::BeforeRendering => started = Some(std::time::Instant::now()),
                slint::RenderingState::AfterRendering => {
                    if let Some(t) = started.take() {
                        let ms = t.elapsed().as_secs_f64() * 1000.0;
                        if ms > 12.0 {
                            eprintln!("[{:.2} s] image lente : {ms:.1} ms", t_start.elapsed().as_secs_f64());
                        }
                    }
                }
                _ => {}
            }
        }
        let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api else { return };
        match rs {
            slint::RenderingState::RenderingSetup => {
                // SAFETY : le contexte OpenGL de la fenêtre est courant pendant ce rappel.
                let gl = unsafe { glow::Context::from_loader_function_cstr(|s| get_proc_address(s)) };
                // SAFETY : contexte courant ; simples lectures de chaînes.
                let info = unsafe {
                    format!("{} · {}", gl.get_parameter_string(glow::VERSION), gl.get_parameter_string(glow::RENDERER))
                };
                let _ = GL_INFO.set(info);
                state = Some(State { gl, render: None, target: None });
            }
            slint::RenderingState::BeforeRendering => {
                if let (Some(st), Some(ui)) = (state.as_mut(), weak.upgrade()) {
                    before_rendering(st, &ui, *get_proc_address);
                }
            }
            slint::RenderingState::RenderingTeardown => {
                if let Some(mut st) = state.take() {
                    st.render = None;
                    free_target(&st.gl, &mut st.target);
                }
            }
            _ => {}
        }
    })
}

fn free_target(gl: &glow::Context, target: &mut Option<Target>) {
    if let Some(t) = target.take() {
        // SAFETY : objets créés par ce même contexte.
        unsafe {
            gl.delete_framebuffer(t.fbo);
            gl.delete_texture(t.tex);
        }
    }
}

fn before_rendering(st: &mut State, ui: &AppWindow, gpa: &dyn Fn(&CStr) -> *const c_void) {
    let mut guard = CURRENT.lock().unwrap();
    let current = guard.as_ref().map(|(m, _)| m.clone());
    let cur_id = current.as_ref().map(|m| Arc::as_ptr(m) as usize);

    if st.render.as_ref().map(|(id, _)| *id) != cur_id {
        // Lecteur changé ou lecture terminée : on libère l'ancien rendu (et, avec lui, le lecteur).
        st.render = None;
        if let (Some(m), Some(id)) = (current, cur_id) {
            let weak = ui.as_weak();
            let on_frame = Box::new(move || {
                let w = weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(u) = w.upgrade() {
                        u.window().request_redraw();
                    }
                });
            });
            match mpv::Render::new(m, gpa, on_frame) {
                Ok(r) => st.render = Some((id, r)),
                Err(e) => eprintln!("turtlefin : {e}"),
            }
            // Prêt (ou en échec : la lecture continue alors sans image plutôt que d'attendre).
            if let Some(tx) = guard.as_mut().and_then(|(_, tx)| tx.take()) {
                let _ = tx.send(());
            }
        } else {
            free_target(&st.gl, &mut st.target);
            ui.set_video_frame(slint::Image::default());
        }
    }

    drop(guard);
    let Some((_, render)) = &st.render else { return };
    let size = ui.window().size();
    if size.width == 0 || size.height == 0 {
        return;
    }
    let gl = &st.gl;

    // SAFETY : contexte OpenGL de la fenêtre courant ; on sauvegarde puis rétablit l'état que
    // mpv modifie, pour que Slint dessine ensuite comme si de rien n'était.
    unsafe {
        let prev_fbo = gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING);
        let mut vp = [0i32; 4];
        gl.get_parameter_i32_slice(glow::VIEWPORT, &mut vp);
        let prev_program = gl.get_parameter_i32(glow::CURRENT_PROGRAM);
        let prev_vao = gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING);
        let prev_buffer = gl.get_parameter_i32(glow::ARRAY_BUFFER_BINDING);
        let prev_active = gl.get_parameter_i32(glow::ACTIVE_TEXTURE);
        let prev_tex = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);
        let blend = gl.is_enabled(glow::BLEND);
        let scissor = gl.is_enabled(glow::SCISSOR_TEST);

        // Texture à la taille de la fenêtre (recréée si elle change).
        let stale = st.target.as_ref().map(|t| (t.w, t.h) != (size.width, size.height)).unwrap_or(true);
        if stale {
            free_target(gl, &mut st.target);
            if let (Ok(tex), Ok(fbo)) = (gl.create_texture(), gl.create_framebuffer()) {
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    size.width as i32,
                    size.height as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
                gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
                st.target = Some(Target { tex, fbo, w: size.width, h: size.height });

                let image = slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                    tex.0,
                    [size.width, size.height].into(),
                )
                .origin(slint::BorrowedOpenGLTextureOrigin::TopLeft)
                .build();
                ui.set_video_frame(image);
            }
        }

        // Diagnostic : TURTLEFIN_DEBUG_NO_VIDEO=1 laisse l'interface se redessiner à chaque image
        // mais sans faire dessiner mpv (pour savoir qui, de mpv ou de Slint, provoque un problème).
        let skip = std::env::var_os("TURTLEFIN_DEBUG_NO_VIDEO").is_some();
        if skip {
            render.acknowledge();
        } else if let Some(t) = &st.target {
            render.render(t.fbo.0.get(), t.w as i32, t.h as i32, stale);
        }

        // Rétablissement de l'état OpenGL attendu par Slint.
        gl.bind_framebuffer(glow::FRAMEBUFFER, NonZeroU32::new(prev_fbo as u32).map(glow::NativeFramebuffer));
        gl.viewport(vp[0], vp[1], vp[2], vp[3]);
        gl.use_program(NonZeroU32::new(prev_program as u32).map(glow::NativeProgram));
        gl.bind_vertex_array(NonZeroU32::new(prev_vao as u32).map(glow::NativeVertexArray));
        gl.bind_buffer(glow::ARRAY_BUFFER, NonZeroU32::new(prev_buffer as u32).map(glow::NativeBuffer));
        gl.active_texture(prev_active as u32);
        gl.bind_texture(glow::TEXTURE_2D, NonZeroU32::new(prev_tex as u32).map(glow::NativeTexture));
        if blend { gl.enable(glow::BLEND) } else { gl.disable(glow::BLEND) }
        if scissor { gl.enable(glow::SCISSOR_TEST) } else { gl.disable(glow::SCISSOR_TEST) }
    }
}
