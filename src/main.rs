extern crate cgmath;
extern crate glfw;
extern crate log;
extern crate stb_image;
extern crate bmfa;

mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

mod gl_help;
mod texture;


use crate::gl::types::{
    GLfloat, GLint, GLsizeiptr, GLuint, GLvoid
};

use crate::gl_help as glh;
use crate::texture::TexImage2D;

use glfw::{Action, Context, Key};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufRead};
use std::mem;
use std::path::Path;
use std::process;
use std::ptr;


// OpenGL extension constants.
const GL_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;
const GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FF;

const ATLAS_PATH: &str = "assets/freemono.bmfa";


struct GameContext {
    gl: glh::GLState,
}

///
/// Load the bitmap font atlas file.
///
fn load_font_atlas<P: AsRef<Path>>(path: P) -> bmfa::BitmapFontAtlas {
    bmfa::load(path).unwrap()
}

///
/// Print a string to the GLFW screen with the given font.
///
fn text_to_vbo(
    context: &glh::GLState, st: &str, atlas: &bmfa::BitmapFontAtlas,
    start_x: f32, start_y: f32, scale_px: f32,
    points_vbo: &mut GLuint, texcoords_vbo: &mut GLuint, point_count: &mut usize) {

    let mut points_temp = vec![0.0; 12 * st.len()];
    let mut texcoords_temp = vec![0.0; 12 * st.len()];
    let mut at_x = start_x;
    let at_y = start_y;

    for (i, ch_i) in st.chars().enumerate() {
        let metadata_i = atlas.glyph_metadata[&(ch_i as usize)];

        // Work out the row and column in the atlas.
        //let atlas_col = (metadata_i.code_point - ' ' as usize) % atlas.columns;
        //let atlas_row = (metadata_i.code_point - ' ' as usize) / atlas.rows;
        let atlas_col = metadata_i.column;
        let atlas_row = metadata_i.row;

        let s = (atlas_col as f32) * (1.0 / (atlas.columns as f32));
        let t = ((atlas_row + 1) as f32) * (1.0 / (atlas.rows as f32));

        let x_pos = at_x;
        let y_pos = at_y - (scale_px / (context.height as f32)) * metadata_i.y_offset;

        at_x +=  metadata_i.width * (scale_px / (context.width as f32));

        points_temp[12 * i]     = x_pos;
        points_temp[12 * i + 1] = y_pos;
        points_temp[12 * i + 2] = x_pos;
        points_temp[12 * i + 3] = y_pos - scale_px / (context.height as f32);
        points_temp[12 * i + 4] = x_pos + scale_px / (context.width as f32);
        points_temp[12 * i + 5] = y_pos - scale_px / (context.height as f32);

        points_temp[12 * i + 6]  = x_pos + scale_px / (context.width as f32);
        points_temp[12 * i + 7]  = y_pos - scale_px / (context.height as f32);
        points_temp[12 * i + 8]  = x_pos + scale_px / (context.width as f32);
        points_temp[12 * i + 9]  = y_pos;
        points_temp[12 * i + 10] = x_pos;
        points_temp[12 * i + 11] = y_pos;

        texcoords_temp[12 * i]     = s;
        texcoords_temp[12 * i + 1] = 1.0 - t + 1.0 / (atlas.rows as f32);
        texcoords_temp[12 * i + 2] = s;
        texcoords_temp[12 * i + 3] = 1.0 - t;
        texcoords_temp[12 * i + 4] = s + 1.0 / (atlas.columns as f32);
        texcoords_temp[12 * i + 5] = 1.0 - t;

        texcoords_temp[12 * i + 6]  = s + 1.0 / (atlas.columns as f32);
        texcoords_temp[12 * i + 7]  = 1.0 - t;
        texcoords_temp[12 * i + 8]  = s + 1.0 / (atlas.columns as f32);
        texcoords_temp[12 * i + 9]  = 1.0 - t + 1.0 / (atlas.rows as f32);
        texcoords_temp[12 * i + 10] = s;
        texcoords_temp[12 * i + 11] = 1.0 - t + 1.0 / (atlas.rows as f32);
    }

    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, *points_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (12 * st.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            points_temp.as_ptr() as *const GLvoid, gl::DYNAMIC_DRAW
        );
        gl::BindBuffer(gl::ARRAY_BUFFER, *texcoords_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (12 * st.len() * mem::size_of::<GLfloat>()) as GLsizeiptr, 
            texcoords_temp.as_ptr() as *const GLvoid, gl::DYNAMIC_DRAW
        );
    }

    *point_count = 6 * st.len();
}

fn create_shaders(context: &mut GameContext) -> (GLuint, GLint) {
    let mut vert_reader = io::Cursor::new(include_str!("../shaders/fontview.vert.glsl"));
    let mut frag_reader = io::Cursor::new(include_str!("../shaders/fontview.frag.glsl"));
    let sp = glh::create_program_from_reader(
        &context.gl,
        &mut vert_reader, "fontview.vert.glsl",
        &mut frag_reader, "fontview.frag.glsl",
    ).unwrap();
    assert!(sp > 0);

    let sp_text_color_loc = unsafe { 
        gl::GetUniformLocation(sp, glh::gl_str("text_color").as_ptr())
    };
    assert!(sp_text_color_loc > 0);

    (sp, sp_text_color_loc)
}

///
/// Load texture image into the GPU.
///
fn load_font_texture(atlas: &bmfa::BitmapFontAtlas, wrapping_mode: GLuint) -> Result<GLuint, String> {
    let mut tex = 0;
    unsafe {
        gl::GenTextures(1, &mut tex);
    }
    assert!(tex > 0);

    unsafe {
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::TexImage2D(
            gl::TEXTURE_2D, 0, gl::RGBA as i32, atlas.width as i32, atlas.height as i32, 0,
            gl::RGBA, gl::UNSIGNED_BYTE,
            atlas.image.as_ptr() as *const GLvoid
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrapping_mode as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrapping_mode as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as GLint);
    }

    let mut max_aniso = 0.0;
    unsafe {
        gl::GetFloatv(GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT, &mut max_aniso);
        // Set the maximum!
        gl::TexParameterf(gl::TEXTURE_2D, GL_TEXTURE_MAX_ANISOTROPY_EXT, max_aniso);
    }

    Ok(tex)
}

///
/// The GLFW frame buffer size callback function. This is normally set using 
/// the GLFW `glfwSetFramebufferSizeCallback` function, but instead we explicitly
/// handle window resizing in our state updates on the application side. Run this function 
/// whenever the size of the viewport changes.
///
#[inline]
fn glfw_framebuffer_size_callback(context: &mut GameContext, width: u32, height: u32) {
    context.gl.width = width;
    context.gl.height = height;
}

fn init_app() -> GameContext {
    let gl_state = match glh::start_gl(800, 480) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Failed to Initialize OpenGL context. Got error:");
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let mut context = GameContext {
        gl: gl_state,
    };

    context
}

fn main() {
    // Start GL context with helper libraries.
    let mut context = init_app();

    // Get renderer string.
    let renderer = glh::glubyte_ptr_to_string(unsafe { gl::GetString(gl::RENDERER) });
    // Get version as a string.
    let version = glh::glubyte_ptr_to_string(unsafe { gl::GetString(gl::VERSION) });
    println!("Renderer: {}", renderer);
    println!("OpenGL version supported {}", version);

    // Load the font atlas.
    let font_atlas = load_font_atlas(ATLAS_PATH);

    // Set a string of text for lower-case letters.
    let mut first_string_vp_vbo = 0;
    unsafe { 
        gl::GenBuffers(1, &mut first_string_vp_vbo);
    }
    assert!(first_string_vp_vbo > 0);
    
    let mut first_string_vt_vbo = 0;
    unsafe { 
        gl::GenBuffers(1, &mut first_string_vt_vbo);
    }
    assert!(first_string_vt_vbo > 0);

    let mut first_string_vao = 0;

    let x_pos: f32 = -0.90;
    let y_pos: f32 = 0.2;
    let pixel_scale = 74.0;
    let first_str = "The Human Torch was denied a bank loan!";
    let mut first_string_points = 0;
    text_to_vbo(
        &context.gl, first_str, &font_atlas, 
        x_pos, y_pos, pixel_scale,
        &mut first_string_vp_vbo, &mut first_string_vt_vbo, &mut first_string_points
    );
    
    unsafe {
        gl::GenVertexArrays(1, &mut first_string_vao);
        gl::BindVertexArray(first_string_vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, first_string_vp_vbo);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, first_string_vt_vbo);
        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(1);
    }

    // Second string of text for capital letters.
    let mut second_string_vp_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut second_string_vp_vbo);
    }
    assert!(second_string_vp_vbo > 0);

    let mut second_string_vt_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut second_string_vt_vbo);
    }
    assert!(second_string_vt_vbo > 0);
    
    let mut second_string_vao = 0;
    let x_pos = -1.0;
    let y_pos = 1.0;
    let pixel_scale = 70.0;
    let second_str = "The human torch was denied a bank loan!";
    let mut second_string_points = 0;
    text_to_vbo(
        &context.gl, second_str, &font_atlas,
        x_pos, y_pos, pixel_scale, 
        &mut second_string_vp_vbo, &mut second_string_vt_vbo, &mut second_string_points
    );
    
    unsafe {
        gl::GenVertexArrays(1, &mut second_string_vao);
        gl::BindVertexArray(second_string_vao );
        gl::BindBuffer(gl::ARRAY_BUFFER, second_string_vp_vbo);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, second_string_vt_vbo);
        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(1);
    }

    let (sp, sp_text_color_loc) = create_shaders(&mut context);

    let tex = load_font_texture(&font_atlas, gl::CLAMP_TO_EDGE).unwrap();;

    unsafe {
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CCW);
        gl::Enable(gl::CULL_FACE);
        // Partial transparency.
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::ClearColor(0.2, 0.2, 0.6, 1.0);
        gl::Viewport(0, 0, context.gl.width as i32, context.gl.height as i32);
    }

    // The main rendering loop.
    while !context.gl.window.should_close() {
        // Check for whether the window size has changed.
        let (width, height) = context.gl.window.get_framebuffer_size();
        if (width != context.gl.width as i32) && (height != context.gl.height as i32) {
            glfw_framebuffer_size_callback(&mut context, width as u32, height as u32);
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::ClearColor(0.2, 0.2, 0.6, 1.0);
            gl::Viewport(0, 0, context.gl.width as i32, context.gl.height as i32);

            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex);
            gl::UseProgram(sp);

            // Draw text with no depth test and alpha blending.
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);

            gl::BindVertexArray(first_string_vao);
            gl::Uniform4f(sp_text_color_loc, 1.0, 0.0, 1.0, 1.0 );
            gl::DrawArrays(gl::TRIANGLES, 0, first_string_points as GLint);

            gl::BindVertexArray(second_string_vao);
            gl::Uniform4f(sp_text_color_loc, 1.0, 1.0, 0.0, 1.0);
            gl::DrawArrays(gl::TRIANGLES, 0, second_string_points as GLint);
        }

        context.gl.glfw.poll_events();
        match context.gl.window.get_key(Key::Escape) {
            Action::Press | Action::Repeat => {
                context.gl.window.set_should_close(true);
            }
            _ => {}
        }
        
        // Send the results to the output.
        context.gl.window.swap_buffers();
    }
}
