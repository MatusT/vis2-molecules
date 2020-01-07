function deg2rad(degrees)
{
  return degrees * (Math.PI / 180);
}

document.addEventListener('DOMContentLoaded', function() {
    let canvas_element = document.getElementById("main-canvas");
    let width = document.body.clientWidth;
    let height = document.body.clientHeight;
    canvas_element.width = width;
    canvas_element.height = height;

    let canvas = canvas_element.getContext("2d", { alpha: false });
    let viewport = canvas.createImageData(width, height);

    const fov = 90.0;
    const scale = Math.tan(deg2rad(fov * 0.5));
    const aspect_ratio = width / height;

    function run(time, last_time) {
        canvas.clearRect(0, 0, width, height);

        let view_matrix = glMatrix.mat4.create();
        let camera_origin = glMatrix.vec4.fromValues(0, 0, 0, 1);
        glMatrix.vec4.transformMat4(camera_origin, camera_origin, view_matrix);

        for(let j = 0; j < height; j++) {
            for(let i = 0; i < width; i++) {                
                let x = (2 * (i + 0.5) / width - 1) * aspect_ratio * scale; 
                let y = (1 - 2 * (j + 0.5) / height) * scale; 

                let direction = glMatrix.vec4.fromValues(x, y, -1, 0);
                glMatrix.vec4.transformMat4(direction, direction, view_matrix);

                let out = glMatrix.vec3.fromValues(1, 1, 0);
                
                let index = 4 * (width * j + i);
                viewport.data[index] = out[0] * 255;
                viewport.data[index + 1] = out[1] * 255;
                viewport.data[index + 2] = out[2] * 255;
                viewport.data[index + 3] = 255;
            }
        }

        canvas.putImageData(viewport, 0, 0);

        requestAnimationFrame(new_time => run(new_time, time));
    }
    requestAnimationFrame(run);
}, false);