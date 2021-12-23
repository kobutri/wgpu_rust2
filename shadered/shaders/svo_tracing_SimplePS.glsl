#version 330

uniform vec2 uResolution;
uniform float uTime;
uniform vec3 camera_pos;
uniform vec3 camera_dir;
uniform vec2 viewport_size;

out vec4 outColor;

const vec3 global_up = vec3(0, 1, 0);
#define PI 3.14159265359

struct Ray {
	vec3 origin;
	vec3 dir;
};

Ray generate_Ray() {
	vec3 camera_dir = normalize(camera_dir);
	vec3 right = cross(camera_dir, global_up);
	vec3 up = cross(right, camera_dir);
	vec2 ratio = vec2(gl_FragCoord) / viewport_size;
	float aspect = viewport_size.x/viewport_size.y;
	float fov = 90*PI/180;
	float a = tan(fov/2.0);
	float a2 = a/aspect;
	vec3 view_center = camera_pos + camera_dir;
	
	vec3 target = view_center + right * a * (2 * ratio.x - 1) + up * a2 * (2 * ratio.y - 1);
	
	return Ray(camera_pos, normalize(target-camera_pos));
}

struct StackNode {
	vec3 origin;
	int index;
};

struct Node {
	int material_id;
	int sub_voxels[8];
};

#define MAX_DEPTH 12
#define EPS 1e-5
const float infinity = 1. / 0.;


const int SOLID = 1;
const int EMPTY = 0;

Node[10] data = Node[](
	Node(0, int[](1, 2, 3, 4, 5, 6, 7, 8)),
	Node(0, int[](9, 0, 0, 0, 0, 0, 0, 0)),
	Node(0, int[](0, 9, 0, 0, 0, 0, 0, 0)),
	Node(0, int[](0, 0, 9, 0, 0, 0, 0, 0)),
	Node(0, int[](0, 0, 0, 9, 0, 0, 0, 0)),
	Node(0, int[](0, 0, 0, 0, 9, 0, 0, 0)),
	Node(0, int[](0, 0, 0, 0, 0, 9, 0, 0)),
	Node(0, int[](0, 0, 0, 0, 0, 0, 9, 0)),
	Node(0, int[](0, 0, 0, 0, 0, 0, 0, 9)),
	Node(1, int[](0, 0, 0, 0, 0, 0, 0, 0))
);

float max3 (vec3 v) {
	return max(max(v.x, v.y), v.z);
}

int inside_voxel(vec3 point, vec3 origin, float size) {
    vec3 dist = (point-origin)/(size);
    bool outside = max3(abs(dist)) > 1;
    if (outside) {
        return -1;
    } else {
        return (dist.x > 0 ? 1 : 0) | (dist.y > 0 ? 2 : 0) | (dist.z > 0 ? 4 : 0);
    }
}

float dist(Ray r, vec3 origin, float size) {
    vec3 t0 = (origin-size-r.origin)/r.dir;
    t0 = vec3(t0.x < 0 ? infinity : t0.x, t0.y < 0 ? infinity : t0.y, t0.z < 0 ? infinity : t0.z);
    vec3 t1 = (origin+size-r.origin)/r.dir;
    t1 = vec3(t1.x < 0 ? infinity : t1.x, t1.y < 0 ? infinity : t1.y, t1.z < 0 ? infinity : t1.z);
    float t = min(t0.x, min(t0.y, min(t0.z, min(t1.x, min(t1.y, t1.z)))));
    return t + EPS;
}

float dist_subvoxel(Ray r, vec3 origin, float size) {
    vec3 t0 = (origin-r.origin)/r.dir;
    t0 = vec3(t0.x < 0 ? infinity : t0.x, t0.y < 0 ? infinity : t0.y, t0.z < 0 ? infinity : t0.z);
    float t1 = dist(r, origin, size);
    float t = min(t1 - EPS, min(t0.x, min(t0.y, t0.z))) + EPS;
    return t;
}

int advance_if_necessary(inout Ray r, vec3 origin, float size) {
    int index = inside_voxel(r.origin, origin, size);
    if (index != -1) {
        return index;
    } else {
        for(int i = 0; i < 2; i++) {
            float t = dist(r, origin, size);
            r.origin += r.dir*t;
            index = inside_voxel(r.origin, origin, size);
            if(index != -1) {
                break;
            }
        }
        return index;
    }
}

int advance_to_next_subvoxel(inout Ray r, vec3 origin, float size) {
    float t = dist_subvoxel(r, origin, size);
    r.origin += r.dir*t;
    return inside_voxel(r.origin, origin, size);
}

void main() {
	float octree_size = 5;

    StackNode stack[MAX_DEPTH];
    int current_stack = 0;
    stack[current_stack] = StackNode(vec3(0.0), 0);

    Ray ray = generate_Ray();


    bool hit = false;
    int i = 0;
    while (i < 100) {
        float size = octree_size/(1 << current_stack);
        int sub_voxel_index = advance_if_necessary(ray, stack[current_stack].origin, size);
        int current_voxel_index = stack[current_stack].index;
        if (sub_voxel_index == -1) {
            if(current_stack == 0) {
                break;
            } else {
                current_stack -= 1;
            }
        } else if (data[current_voxel_index].material_id == SOLID) {
            hit = true;
            break;
        } else if (data[current_voxel_index].sub_voxels[sub_voxel_index] != 0){
            vec3 old_origin = stack[current_stack].origin;
            current_stack += 1;
            stack[current_stack].origin = old_origin + vec3((sub_voxel_index & 1) != 0 ? 1 : -1, (sub_voxel_index & 2) != 0 ? 1: -1, (sub_voxel_index & 4) != 0 ? 1 : -1) * size/2;
            stack[current_stack].index = data[current_voxel_index].sub_voxels[sub_voxel_index];
        } else if(advance_to_next_subvoxel(ray, stack[current_stack].origin, size) == -1) {
        	if(current_stack == 0) {
		        break;
            } else {
            	current_stack -= 1;
            }
        }
        i++;
    }
//
//    int index = -1;
//    for(int i = 0; i < 2; i++) {
//        float t = dist(ray, stack[current_stack].origin, uniforms.octree_size/(1 << (current_stack)));
//        ray.origin += ray.dir*t;
//        index = inside_voxel(ray.origin, stack[current_stack].origin, uniforms.octree_size/(1 << (current_stack)));
//        if(index != -1) {
//            break;
//        }
//    }
//    hit = index != -1;

    outColor = vec4(vec3(hit), 1);
}