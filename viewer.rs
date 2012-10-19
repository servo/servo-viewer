extern mod glut;
extern mod io_surface;
extern mod opengles;

use mod opengles::cgl;
use mod opengles::gl2;
use io_surface::{IOSurface, IOSurfaceID};
use opengles::gl2::{GLenum, GLint, GLsizei, GLuint};

use cast::transmute;
use from_str::from_str;
use libc::c_int;
use pipes::SharedChan;

fn fragment_shader_source() -> ~str {
    ~"
    #ifdef GLES2
        precision mediump float;
    #endif

        varying vec2 vTextureCoord;

        uniform sampler2DRect uSampler;

        void main(void) {
            gl_FragColor = texture2DRect(uSampler, vec2(vTextureCoord.s, vTextureCoord.t));
        }
    "
}

fn vertex_shader_source() -> ~str {
    ~"
        attribute vec3 aVertexPosition;
        attribute vec2 aTextureCoord;

        varying vec2 vTextureCoord;

        void main(void) {
            gl_Position = vec4(aVertexPosition, 1.0);
            vTextureCoord = aTextureCoord;
        }
    "
}

fn load_shader(source_str: ~str, shader_type: GLenum) -> GLuint {
    let shader_id = gl2::create_shader(shader_type);
    gl2::shader_source(shader_id, ~[str::to_bytes(source_str)]);
    gl2::compile_shader(shader_id);

    if gl2::get_error() != gl2::NO_ERROR {
        io::println(#fmt("error: %d", gl2::get_error() as int));
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
    aVertexPosition: c_int,
    aTextureCoord: c_int,
    uSampler: c_int
}

fn shader_program(program: GLuint) -> shader_program {
    let aVertexPosition = gl2::get_attrib_location(program, ~"aVertexPosition");
    let aTextureCoord = gl2::get_attrib_location(program, ~"aTextureCoord");
    gl2::enable_vertex_attrib_array(aVertexPosition as GLuint);
    gl2::enable_vertex_attrib_array(aTextureCoord as GLuint);

    shader_program {
        program: program,
        aVertexPosition: aVertexPosition,
        aTextureCoord: aTextureCoord,
        uSampler: gl2::get_uniform_location(program, ~"uSampler")
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

fn init_buffers() -> (GLuint, GLuint) {
    let triangle_vertex_buffer = gl2::gen_buffers(1 as GLsizei)[0];
    gl2::bind_buffer(gl2::ARRAY_BUFFER, triangle_vertex_buffer);

    let (n1, _0, _1) = (-1.0f32, 0.0f32, 1.0f32);
    let vertices = ~[
        n1, _1, _0,
        _1, _1, _0,
        n1, n1, _0,
        _1, n1, _0,
    ];
    gl2::buffer_data(gl2::ARRAY_BUFFER, vertices, gl2::STATIC_DRAW);

    let texture_coord_buffer = gl2::gen_buffers(1 as GLsizei)[0];
    gl2::bind_buffer(gl2::ARRAY_BUFFER, texture_coord_buffer);

    let (_800, _600) = (800.0f32, 600.0f32);
    let vertices = ~[
        _0,   _600,
        _800, _600,
        _0,   _0,
        _800, _0
    ];

    gl2::buffer_data(gl2::ARRAY_BUFFER, vertices, gl2::STATIC_DRAW);

    return (triangle_vertex_buffer, texture_coord_buffer);
}

fn draw_scene(shader_program: shader_program, vertex_buffer: GLuint, texture_coord_buffer: GLuint,
              texture: GLuint) {
    gl2::enable(gl2::TEXTURE_2D);
    gl2::enable(gl2::TEXTURE_RECTANGLE_ARB);

    gl2::clear_color(1.0f32, 1.0f32, 1.0f32, 1.0f32);
    gl2::clear(gl2::COLOR_BUFFER_BIT);

    gl2::bind_texture(gl2::TEXTURE_RECTANGLE_ARB, texture);

    gl2::bind_buffer(gl2::ARRAY_BUFFER, vertex_buffer);
    gl2::vertex_attrib_pointer_f32(shader_program.aVertexPosition as GLuint, 3, false, 0, 0);

    gl2::bind_buffer(gl2::ARRAY_BUFFER, texture_coord_buffer);
    gl2::vertex_attrib_pointer_f32(shader_program.aTextureCoord as GLuint, 2, false, 0, 0);

    gl2::uniform_1i(shader_program.uSampler, 0);

    gl2::draw_arrays(gl2::TRIANGLE_STRIP, 0 as GLint, 4 as GLint);
}

fn init_texture(surface: &IOSurface) -> GLuint {
    let texture = gl2::gen_textures(1)[0];
    gl2::bind_texture(gl2::TEXTURE_RECTANGLE_ARB, texture);

    gl2::tex_parameter_i(gl2::TEXTURE_RECTANGLE_ARB, gl2::TEXTURE_WRAP_S,
                         gl2::CLAMP_TO_EDGE as GLint);
    gl2::tex_parameter_i(gl2::TEXTURE_RECTANGLE_ARB, gl2::TEXTURE_WRAP_T,
                         gl2::CLAMP_TO_EDGE as GLint);
    gl2::tex_parameter_i(gl2::TEXTURE_RECTANGLE_ARB, gl2::TEXTURE_MAG_FILTER,
                         gl2::LINEAR as GLint);
    gl2::tex_parameter_i(gl2::TEXTURE_RECTANGLE_ARB, gl2::TEXTURE_MIN_FILTER,
                         gl2::LINEAR as GLint);

    unsafe {
        let cgl_context = cgl::CGLGetCurrentContext();
        assert 0 != transmute(copy cgl_context);

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
    }

    return texture;
}

fn display_callback(context: &Context) {
    let program = init_shaders();
    let (vertex_buffer, texture_coord_buffer) = init_buffers();

    let texture;
    match context.texture {
        None => {
            let tex = init_texture(&context.surface);
            texture = tex;
            context.texture = Some(texture);
        }
        Some(copy tex) => {
            texture = tex;
        }
    }

    draw_scene(program, vertex_buffer, texture_coord_buffer, texture);

    glut::swap_buffers();
}

struct Context {
    surface: IOSurface,
    mut texture: Option<GLuint>
}

fn start_app(finish_chan: &SharedChan<()>, surface_id: IOSurfaceID) {
    glut::init();
    glut::init_display_mode(0);
    let window = glut::create_window(~"Servo Viewer");

    let context = ~Context {
        surface: IOSurface::lookup(surface_id),
        texture: None
    };

    do glut::display_func |move context| {
        display_callback(context);
    }

    loop {
        glut::check_loop();
    }
}

fn main() {
    let args = os::args();
    let surface_id: int = from_str(args[1]).get();

    let (finish_chan, finish_port) = pipes::stream();
    let finish_chan = SharedChan(move finish_chan);
    do task::task().sched_mode(task::PlatformThread).spawn |move finish_chan| {
        start_app(&finish_chan, surface_id as IOSurfaceID);
    }
    finish_port.recv();
}


