

use cgmath::{self, num_traits::Pow, EuclideanSpace, Rotation3, Vector3};

use crate::math::OPENGL_TO_WGPU_MATRIX;



pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 1.
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // 2.
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        
        // 3.
        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    

}



#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }

    pub fn rotate(&mut self,camera: &Camera ,pitch: f32, _yaw: f32) {
        self.view_proj = (camera.build_view_projection_matrix() * cgmath::Matrix4::from_angle_x(cgmath::Deg(pitch))).into();


    }

}

 
pub struct CameraStaging {
    pub camera: Camera,
    pub model_rotation: cgmath::Deg<f32>,
}

impl CameraStaging {
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            model_rotation: cgmath::Deg(0.0),
        }
    }
    pub fn update_camera(&mut self, camera_uniform: &mut CameraUniform) {
        

        camera_uniform.view_proj = (self.camera.build_view_projection_matrix()
        ).into();
    }

    pub fn update_camera_pitch(&mut self, camera_uniform: &mut CameraUniform) {
        let v = self.camera.target - self.camera.eye;
        let pitch = self.camera.pitch;
        eprintln!("Sine of pitch: {}", pitch.sin());
        let new_p = cgmath::Point3::new(
            self.camera.target.x,
            v.y * pitch.cos() - v.z * pitch.sin(),
            v.y * pitch.sin() + v.z * pitch.cos()
        );
        
        //eprintln!("camera target: {:?}", new_p.to_vec());
        self.camera.target = new_p;
        self.camera.pitch = 0.0;
        camera_uniform.view_proj = (self.camera.build_view_projection_matrix()
        ).into();
    }

    pub fn update_camera_yaw(&mut self, camera_uniform: &mut CameraUniform) {
        let v = self.camera.target - self.camera.eye;
        let pitch = self.camera.yaw;
        eprintln!("Sine of pitch: {}", pitch.sin());
        let new_p = cgmath::Point3::new(
            v.x * pitch.cos() + v.z * pitch.cos(),
            v.y,
            -v.x * pitch.cos() + v.z * pitch.cos()
        );
        
        //eprintln!("camera target: {:?}", new_p.to_vec());
        self.camera.target = new_p;
        self.camera.yaw = 0.0;
        camera_uniform.view_proj = (self.camera.build_view_projection_matrix()
        ).into();
    }

}
 
