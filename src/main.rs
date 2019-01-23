extern crate glfw;
extern crate log;
extern crate stb_image;
extern crate bmfa;
extern crate structopt;

mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

mod gl_help;


use crate::gl::types::{
    GLfloat, GLint, GLsizeiptr, GLuint, GLvoid
};

use crate::gl_help as glh;

use glfw::{Action, Context, Key};
use std::cell::{Ref, RefMut, RefCell};
use std::fmt;
use std::io;
use std::io::Write;
use std::mem;
use std::path::{Path, PathBuf};
use std::process;
use std::ptr;
use std::rc::Rc;
use std::str;
use structopt::StructOpt;


// OpenGL extension constants.
const GL_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;
const GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FF;

const ATLAS_PATH: &str = "assets/freemono.bmfa";

const DEFAULT_TEXT: &str = "\
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor \
incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis \
nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. \
Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu \
fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in \
culpa qui officia deserunt mollit anim id est laborum.";


struct GameContext {
    gl: Rc<RefCell<glh::GLState>>,
}

impl GameContext {
    #[inline]
    fn gl(&self) -> Ref<glh::GLState> {
        self.gl.borrow()
    }

    #[inline]
    fn gl_mut(&self) -> RefMut<glh::GLState> {
        self.gl.borrow_mut()
    }
}

///
/// Load the bitmap font atlas file.
///
fn load_font_atlas<P: AsRef<Path>>(path: P) -> Rc<bmfa::BitmapFontAtlas> {
    let atlas = bmfa::load(path).unwrap();

    Rc::new(atlas)
}

struct TextWriter {
    context: Rc<RefCell<glh::GLState>>,
    atlas: Rc<bmfa::BitmapFontAtlas>,
    start_at_x: f32,
    start_at_y: f32,
    scale_px: f32,
    point_count: usize,
    writer: GLTextWriter,
}

impl TextWriter {
    fn new(
        context: Rc<RefCell<glh::GLState>>,
        atlas: Rc<bmfa::BitmapFontAtlas>,
        start_at_x: f32, start_at_y: f32, scale_px: f32,
        writer: GLTextWriter) -> TextWriter {

        TextWriter {
            context: context,
            atlas: atlas,
            start_at_x: start_at_x,
            start_at_y: start_at_y,
            scale_px: scale_px,
            point_count: 0,
            writer: writer,
        }
    }

    fn point_count(&self) -> usize {
        self.point_count
    }
}

impl io::Write for TextWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let st = str::from_utf8(buf).unwrap();
        let atlas = &self.atlas;
        let scale_px = self.scale_px;
        let height = (*self.context).borrow().height;
        let width = (*self.context).borrow().width;
        let line_spacing = 0.05;

        let mut points = vec![0.0; 12 * st.len()];
        let mut texcoords = vec![0.0; 12 * st.len()];
        let mut at_x = self.start_at_x;
        let end_at_x = 0.95;
        let mut at_y = self.start_at_y;

        for (i, ch_i) in st.chars().enumerate() {
            let metadata_i = atlas.glyph_metadata[&(ch_i as usize)];
            let atlas_col = metadata_i.column;
            let atlas_row = metadata_i.row;

            let s = (atlas_col as f32) * (1.0 / (atlas.columns as f32));
            let t = ((atlas_row + 1) as f32) * (1.0 / (atlas.rows as f32));

            let x_pos = at_x;
            let y_pos = at_y - (scale_px / (height as f32)) * metadata_i.y_offset;

            at_x += metadata_i.width * (scale_px / width as f32);
            if at_x >= end_at_x {
                at_x = self.start_at_x;
                at_y -= line_spacing + metadata_i.height * (scale_px / height as f32);
            }

            points[12 * i]     = x_pos;
            points[12 * i + 1] = y_pos;
            points[12 * i + 2] = x_pos;
            points[12 * i + 3] = y_pos - scale_px / (height as f32);
            points[12 * i + 4] = x_pos + scale_px / (width as f32);
            points[12 * i + 5] = y_pos - scale_px / (height as f32);

            points[12 * i + 6]  = x_pos + scale_px / (width as f32);
            points[12 * i + 7]  = y_pos - scale_px / (height as f32);
            points[12 * i + 8]  = x_pos + scale_px / (width as f32);
            points[12 * i + 9]  = y_pos;
            points[12 * i + 10] = x_pos;
            points[12 * i + 11] = y_pos;

            texcoords[12 * i]     = s;
            texcoords[12 * i + 1] = 1.0 - t + 1.0 / (atlas.rows as f32);
            texcoords[12 * i + 2] = s;
            texcoords[12 * i + 3] = 1.0 - t;
            texcoords[12 * i + 4] = s + 1.0 / (atlas.columns as f32);
            texcoords[12 * i + 5] = 1.0 - t;

            texcoords[12 * i + 6]  = s + 1.0 / (atlas.columns as f32);
            texcoords[12 * i + 7]  = 1.0 - t;
            texcoords[12 * i + 8]  = s + 1.0 / (atlas.columns as f32);
            texcoords[12 * i + 9]  = 1.0 - t + 1.0 / (atlas.rows as f32);
            texcoords[12 * i + 10] = s;
            texcoords[12 * i + 11] = 1.0 - t + 1.0 / (atlas.rows as f32);
        }

        self.point_count = 6 * st.len();
        self.writer.write(&points, &texcoords)?;

        Ok(st.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct GLTextWriter {
    vao: GLuint,
    points_vbo: GLuint,
    texcoords_vbo: GLuint,
}

impl GLTextWriter {
    fn new(vao: GLuint, points_vbo: GLuint, texcoords_vbo: GLuint) -> GLTextWriter {
        GLTextWriter {
            vao: vao,
            points_vbo: points_vbo,
            texcoords_vbo: texcoords_vbo,
        }
    }

    fn write(&mut self, points: &[GLfloat], texcoords: &[GLfloat]) -> io::Result<usize> {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.points_vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER, (mem::size_of::<GLfloat>() * points.len()) as GLsizeiptr,
                points.as_ptr() as *const GLvoid, gl::DYNAMIC_DRAW
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, self.texcoords_vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER, (mem::size_of::<GLfloat>() * texcoords.len()) as GLsizeiptr,
                texcoords.as_ptr() as *const GLvoid, gl::DYNAMIC_DRAW
            );
        }

        let bytes_written = mem::size_of::<GLfloat>() * (points.len() + texcoords.len());

        Ok(bytes_written)
    }
}

fn create_shaders(context: &mut GameContext) -> (GLuint, GLint) {
    let mut vert_reader = io::Cursor::new(include_str!("../shaders/fontview.vert.glsl"));
    let mut frag_reader = io::Cursor::new(include_str!("../shaders/fontview.frag.glsl"));
    let sp = glh::create_program_from_reader(
        &context.gl(),
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

fn create_text_writer(
    context: &mut GameContext,
    atlas: Rc<bmfa::BitmapFontAtlas>) -> (TextWriter, GLuint, GLuint, GLuint) {

    let mut points_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut points_vbo);
    }
    assert!(points_vbo > 0);

    let mut texcoords_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut texcoords_vbo);
    }
    assert!(texcoords_vbo > 0);

    let mut vao = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
    }
    assert!(vao > 0);

    let start_at_x = -0.95;
    let start_at_y = 0.95;
    let scale_px = 64.0;
    let gl_writer = GLTextWriter::new(vao, points_vbo, texcoords_vbo);
    let writer = TextWriter::new(
        context.gl.clone(), atlas.clone(), start_at_x, start_at_y, scale_px, gl_writer
    );

    (writer, vao, points_vbo, texcoords_vbo)
}

///
/// The GLFW frame buffer size callback function. This is normally set using
/// the GLFW `glfwSetFramebufferSizeCallback` function; instead we explicitly
/// handle window resizing in our state updates on the application side. Run this function
/// whenever the size of the viewport changes.
///
#[inline]
fn glfw_framebuffer_size_callback(context: &mut GameContext, width: u32, height: u32) {
    context.gl_mut().width = width;
    context.gl_mut().height = height;
}

#[derive(Clone, Debug)]
enum OptError {
    InputFileDoesNotExist(PathBuf),
}

impl fmt::Display for OptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OptError::InputFileDoesNotExist(ref path) => {
                write!(f, "The font file {} could not be found.", path.display())
            }
        }
    }
}

///
/// The shell input options for `fontview`.
///
#[derive(Debug, StructOpt)]
#[structopt(name = "fontview")]
#[structopt(about = "A shell utility for view bitmapped font atlas files.")]
struct Opt {
    /// The path to the input file.
    #[structopt(parse(from_os_str))]
    #[structopt(short = "i", long = "input")]
    input_path: PathBuf,
}

///
/// Verify the input options.
///
fn verify_opt(opt: &Opt) -> Result<(), OptError> {
    if !(opt.input_path.exists() && opt.input_path.is_file()) {
        return Err(OptError::InputFileDoesNotExist(opt.input_path.clone()));
    }

    Ok(())
}

fn init_app() -> GameContext {
    let gl_state = match glh::start_gl(1024, 576) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Failed to Initialize OpenGL context. Got error:");
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let context = GameContext {
        gl: Rc::new(RefCell::new(gl_state)),
    };

    context
}

fn main() {
    // Parse the shell arguments.
    let opt = Opt::from_args();
    match verify_opt(&opt) {
        Err(e) => {
            eprintln!("Error: {:?}", e);
            process::exit(1);
        }
        Ok(_) => {}
    }

    // Start GL context with helper libraries.
    let mut context = init_app();

    // Get renderer string.
    let renderer = glh::glubyte_ptr_to_string(unsafe { gl::GetString(gl::RENDERER) });
    // Get version as a string.
    let version = glh::glubyte_ptr_to_string(unsafe { gl::GetString(gl::VERSION) });
    println!("Renderer: {}", renderer);
    println!("OpenGL version supported {}", version);

    // Load the font atlas.
    let atlas = load_font_atlas(ATLAS_PATH);

    // Create the text writer.
    let (
        mut writer,
        string_vao,
        string_vp_vbo,
        string_vt_vbo) = create_text_writer(&mut context, atlas.clone());

    // Write out the lorem ipsum text.
    let string = DEFAULT_TEXT;
    write!(writer, "{}", string).unwrap();

    unsafe {
        gl::BindVertexArray(string_vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, string_vp_vbo);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, string_vt_vbo);
        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(1);
    }

    let (sp, sp_text_color_loc) = create_shaders(&mut context);

    let tex = load_font_texture(&atlas, gl::CLAMP_TO_EDGE).unwrap();;

    unsafe {
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CCW);
        gl::Enable(gl::CULL_FACE);
        // Partial transparency.
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::ClearColor(0.2, 0.2, 0.6, 1.0);
        gl::Viewport(0, 0, context.gl().width as i32, context.gl().height as i32);
    }

    // The main rendering loop.
    while !context.gl().window.should_close() {
        // Check for whether the window size has changed.
        let (width, height) = context.gl().window.get_framebuffer_size();
        if (width != context.gl().width as i32) && (height != context.gl().height as i32) {
            glfw_framebuffer_size_callback(&mut context, width as u32, height as u32);
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::ClearColor(0.2, 0.2, 0.6, 1.0);
            gl::Viewport(0, 0, context.gl().width as i32, context.gl().height as i32);

            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex);
            gl::UseProgram(sp);

            // Draw text with no depth test and alpha blending.
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);

            gl::BindVertexArray(string_vao);
            gl::Uniform4f(sp_text_color_loc, 1.0, 1.0, 0.0, 1.0);
            gl::DrawArrays(gl::TRIANGLES, 0, writer.point_count() as GLint);
        }

        context.gl_mut().glfw.poll_events();
        match context.gl().window.get_key(Key::Escape) {
            Action::Press | Action::Repeat => {
                context.gl_mut().window.set_should_close(true);
            }
            _ => {}
        }
        
        // Send the results to the output.
        context.gl_mut().window.swap_buffers();
    }
}
