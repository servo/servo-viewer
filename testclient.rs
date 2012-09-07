extern mod core_foundation;
extern mod io_surface;
extern mod opengles;

use mod opengles::cgl;
use mod opengles::gl2;
use core_foundation::base::CFTypeOps;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use libc::c_int;
use opengles::gl2::{GLenum, GLint, GLsizei, GLuint};
use pipes::SharedChan;
use unsafe::transmute;

fn fragment_shader_source() -> ~str {
    ~"
    #ifdef GLES2
        precision mediump float;
    #endif

        void main(void) {
            gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
        }
    "
}

fn vertex_shader_source() -> ~str {
    ~"
        attribute vec3 aVertexPosition;

        /*uniform mat4 uMVMatrix;
        uniform mat4 uPMatrix;*/

        void main(void) {
            gl_Position = /*uPMatrix * uMVMatrix **/
                vec4(aVertexPosition, 1.0);
        }
    "
}

fn load_shader(source_str: ~str, shader_type: GLenum) -> GLuint {
    let shader_id = gl2::create_shader(shader_type);
    io::println(fmt!("shader id %?", shader_id));
    gl2::shader_source(shader_id, ~[str::to_bytes(source_str)]);
    gl2::compile_shader(shader_id);

    let gl_error = gl2::get_error();
    if gl_error != gl2::NO_ERROR {
        io::println(#fmt("error: %d", gl_error as int));
        fail ~"failed to compile shader with error";
    }

    if gl2::get_shader_iv(shader_id, gl2::COMPILE_STATUS) == (0 as GLint) {
        io::println(gl2::get_shader_info_log(shader_id));
        fail ~"failed to compile shader";
    }
    return shader_id;
}

struct shader_program {
    let program: GLuint;
    let aVertexPosition: c_int;

    new(program: GLuint) {
        self.program = program;
        self.aVertexPosition = gl2::get_attrib_location(program, ~"aVertexPosition");

        gl2::enable_vertex_attrib_array(self.aVertexPosition as GLuint);
    }
}

fn init_shaders() -> shader_program {
    let vertex_shader = load_shader(vertex_shader_source(), gl2::VERTEX_SHADER);
    let fragment_shader = load_shader(fragment_shader_source(), gl2::FRAGMENT_SHADER);

    let program = gl2::create_program();
    gl2::attach_shader(program, vertex_shader);
    gl2::attach_shader(program, fragment_shader);
    gl2::link_program(program);

    if gl2::get_program_iv(program, gl2::LINK_STATUS) == (0 as GLint) {
        fail ~"failed to initialize program";
    }

    gl2::use_program(program);

    return shader_program(program);
}

fn init_buffers() -> GLuint {
    let triangle_vertex_buffer = gl2::gen_buffers(1 as GLsizei)[0];
    gl2::bind_buffer(gl2::ARRAY_BUFFER, triangle_vertex_buffer);
    let vertices = ~[
        0.0f32, 1.0f32, 0.0f32,
        1.0f32, 0.0f32, 0.0f32,
        0.0f32, 0.0f32, 0.0f32
    ];
    gl2::buffer_data(gl2::ARRAY_BUFFER, vertices, gl2::STATIC_DRAW);
    return triangle_vertex_buffer;
}

fn draw_scene(shader_program: shader_program, vertex_buffer: GLuint) {
    gl2::clear_color(0.0f32, 0.0f32, 1.0f32, 1.0f32);
    gl2::clear(gl2::COLOR_BUFFER_BIT);

    gl2::bind_buffer(gl2::ARRAY_BUFFER, vertex_buffer);
    gl2::vertex_attrib_pointer_f32(shader_program.aVertexPosition as GLuint, 3 as GLint, false,
                                   0 as GLsizei, 0 as GLuint);
    gl2::draw_arrays(gl2::TRIANGLE_STRIP, 0 as GLint, 3 as GLint);
}

fn display_callback() {
    let program = init_shaders();
    let vertex_buffer = init_buffers();
    draw_scene(program, vertex_buffer);

    gl2::finish();
}

#[link_args="-framework IOSurface"]
#[nolink]
extern {
}

#[link_args="-framework CoreFoundation"]
#[nolink]
extern {
}

fn start_app(finish_chan: SharedChan<()>) unsafe {
    // Choose a pixel format.
    let attributes = [ 
        cgl::kCGLPFADoubleBuffer,
        cgl::kCGLPFACompliant,
        0
    ];
    let pixel_format_obj = ptr::null();
    let pixel_format_count = 1;
    let gl_error = cgl::CGLChoosePixelFormat(transmute(&attributes[0]),
                                             ptr::to_unsafe_ptr(&pixel_format_obj),
                                             ptr::to_unsafe_ptr(&pixel_format_count));
    assert gl_error == cgl::kCGLNoError;

    // Create the context.
    let cgl_context = transmute(0);
    let gl_error = cgl::CGLCreateContext(pixel_format_obj, transmute(0), transmute(&cgl_context));
    assert gl_error == cgl::kCGLNoError;

    // Set the context.
    let gl_error = cgl::CGLSetCurrentContext(cgl_context);
    assert gl_error == cgl::kCGLNoError;

    // Lock the context.
    let gl_error = cgl::CGLLockContext(cgl_context);
    assert gl_error == cgl::kCGLNoError;

    // Create an IOSurface.
    let surface = io_surface::IOSurface::new_io_surface(&CFDictionary::new_dictionary([
        (CFString::wrap(io_surface::kIOSurfaceWidth),
            (&CFNumber::new_number(800i32)).as_type()),
        (CFString::wrap(io_surface::kIOSurfaceHeight),
            (&CFNumber::new_number(600i32)).as_type()),
        (CFString::wrap(io_surface::kIOSurfaceBytesPerRow),
            (&CFNumber::new_number(800i32 * 4)).as_type()),
        (CFString::wrap(io_surface::kIOSurfaceBytesPerElement),
            (&CFNumber::new_number(4i32)).as_type()),
        (CFString::wrap(io_surface::kIOSurfaceIsGlobal),
            (&CFBoolean::true_value()).as_type())
    ]));

    io::println(fmt!("surface id is %d", surface.get_id() as int));

    // Create a framebuffer.
    let ids = gl2::gen_framebuffers(1);
    let id = ids[0];
    gl2::bind_framebuffer(gl2::FRAMEBUFFER, id);

    // Create textures.
    gl2::enable(gl2::TEXTURE_RECTANGLE_ARB);
    let texture = gl2::gen_textures(1)[0];
    gl2::bind_texture(gl2::TEXTURE_RECTANGLE_ARB, texture);
    gl2::tex_parameter_i(gl2::TEXTURE_RECTANGLE_ARB, gl2::TEXTURE_WRAP_S, gl2::CLAMP_TO_EDGE as GLint);
    gl2::tex_parameter_i(gl2::TEXTURE_RECTANGLE_ARB, gl2::TEXTURE_WRAP_T, gl2::CLAMP_TO_EDGE as GLint);
    gl2::tex_parameter_i(gl2::TEXTURE_RECTANGLE_ARB, gl2::TEXTURE_MAG_FILTER, gl2::LINEAR as GLint);
    gl2::tex_parameter_i(gl2::TEXTURE_RECTANGLE_ARB, gl2::TEXTURE_MIN_FILTER, gl2::NEAREST as GLint);

    // Bind to the texture.
    let gl_error = cgl::CGLTexImageIOSurface2D(cgl_context,
                                               gl2::TEXTURE_RECTANGLE_ARB,
                                               gl2::RGBA as GLenum,
                                               800,
                                               600,
                                               gl2::BGRA as GLenum,
                                               gl2::UNSIGNED_INT_8_8_8_8_REV,
                                               transmute(copy surface.obj),
                                               0);
    assert gl_error == cgl::kCGLNoError;
    let status = gl2::get_error();
    if status != gl2::NO_ERROR {
        fail fmt!("failed CGLTexImageIOSurface2D: %x", status as uint);
    }

    /*gl2::tex_image_2d(gl2::TEXTURE_RECTANGLE_ARB, 0, gl2::RGBA as GLint, 800, 600, 0,
                      gl2::BGRA as GLenum, gl2::UNSIGNED_INT_8_8_8_8_REV, None);
    let status = gl2::get_error();
    if status != gl2::NO_ERROR {
        fail fmt!("failed tex_image_2d: %x", status as uint);
    }*/

    // Bind the texture to the framebuffer.
    gl2::bind_texture(gl2::TEXTURE_RECTANGLE_ARB, 0);
    gl2::framebuffer_texture_2d(gl2::FRAMEBUFFER, gl2::COLOR_ATTACHMENT0,
                                gl2::TEXTURE_RECTANGLE_ARB, texture, 0);
    let status = gl2::check_framebuffer_status(gl2::FRAMEBUFFER);
    if status != gl2::FRAMEBUFFER_COMPLETE {
        fail fmt!("failed framebuffer_texture_2d: %x", status as uint);
    }

    debug!("bound framebuffer");

    display_callback();

    let (_, port): (pipes::Chan<()>, pipes::Port<()>) = pipes::stream();
    port.recv();
}

fn main() {
    let (finish_chan, finish_port) = pipes::stream();
    let finish_chan = SharedChan(finish_chan);
    do task::task().sched_mode(task::PlatformThread).spawn {
        start_app(finish_chan);
    }
    finish_port.recv();
}

