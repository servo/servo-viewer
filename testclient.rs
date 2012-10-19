extern mod core_foundation;
extern mod geom;
extern mod io_surface;
extern mod opengles;
extern mod sharegl;

use mod opengles::gl2;
use cast::transmute;
use geom::size::Size2D;
use libc::c_int;
use opengles::gl2::{GLenum, GLint, GLsizei, GLuint};
use pipes::SharedChan;
use sharegl::base::ShareContext;

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

        void main(void) {
            gl_Position = vec4(aVertexPosition, 1.0);
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
    program: GLuint,
    aVertexPosition: c_int
}

fn shader_program(program: GLuint) -> shader_program {
    let aVertexPosition = gl2::get_attrib_location(program, ~"aVertexPosition");
    gl2::enable_vertex_attrib_array(aVertexPosition as GLuint);
    
    shader_program { program: program, aVertexPosition: aVertexPosition }
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
    gl2::viewport(0, 0, 800, 600);

    let program = init_shaders();
    let vertex_buffer = init_buffers();
    draw_scene(program, vertex_buffer);
}

#[link_args="-framework IOSurface"]
#[nolink]
extern {
}

#[link_args="-framework CoreFoundation"]
#[nolink]
extern {
}

fn start_app(finish_chan: &SharedChan<()>) unsafe {
    let share_context: sharegl::platform::Context = sharegl::base::new(Size2D(800, 600));
    io::println(fmt!("ID is %d", share_context.id()));
    display_callback();
    share_context.flush();

    let (_, port): (pipes::Chan<()>, pipes::Port<()>) = pipes::stream();
    port.recv();
}

fn main() {
    let (finish_chan, finish_port) = pipes::stream();
    let finish_chan = SharedChan(finish_chan);
    do task::task().sched_mode(task::PlatformThread).spawn |move finish_chan| {
        start_app(&finish_chan);
    }
    finish_port.recv();
}

