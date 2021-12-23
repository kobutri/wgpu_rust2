#version 450

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outColorDebug;

layout(std140, set = 0, binding = 0) uniform Uniforms {
    vec3 view_pos;
    vec3 view_dir;
    vec3 view_up;
    vec3 view_right;
    float fov;
    uint width;
    uint height;
    float octree_size;
    uint octree_depth;
} uniforms;

struct Node {
    uint material_id;
    uint sub_voxels[8];
};

layout(std430, set = 1, binding = 0) buffer octree {
    Node data[];
};

#define MAX_DEPTH 12
#define EPS 1e-5
const float infinity = 1. / 0.;


const uint SOLID = 1;
const uint EMPTY = 0;

struct Ray {
    vec3 origin;
    vec3 dir;
};

Ray generate_ray()  {
    float x_ratio = float(gl_FragCoord.x) / float(uniforms.width);
    float y_ratio = float(gl_FragCoord.y) / float(uniforms.height);
    float aspect = float(uniforms.width) / float(uniforms.height);
    float a = tan(uniforms.fov/2.0);
    float a2 = a/aspect;
    vec3 view_center = uniforms.view_pos + uniforms.view_dir;

    vec3 target = view_center + uniforms.view_right * a * (2 * x_ratio - 1) + uniforms.view_up * a2 * (2 * y_ratio - 1);

    return Ray(uniforms.view_pos, normalize(target-uniforms.view_pos));
}

struct StackNode {
    vec3 origin;
    uint index;
};

int inside_voxel(vec3 point, vec3 origin, float size) {
    vec3 dist = (point-origin)/(size+EPS);
    int res = 0;
    if (abs(dist.x) > 1 || abs(dist.y) > 1 || abs(dist.z) > 1) {
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
    StackNode stack[MAX_DEPTH];
    uint current_stack = 0;
    stack[current_stack] = StackNode(vec3(0.0), 0);

    Ray ray = generate_ray();


    bool hit = false;
    float depth = 0;
    bool been_there = false;
    int i = 0;
    while (i < 100) {
        float size = uniforms.octree_size/(1 << current_stack);
        int sub_voxel_index = advance_if_necessary(ray, stack[current_stack].origin, size);
        uint current_voxel_index = stack[current_stack].index;
        depth = current_stack;
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
            stack[current_stack].origin = old_origin + ((sub_voxel_index & 1) != 0 ? 1 : -1, (sub_voxel_index & 2) != 0 ? 1: -1, (sub_voxel_index & 4) != 0 ? 1 : -1) * size/2;
            stack[current_stack].index = data[current_voxel_index].sub_voxels[sub_voxel_index];
            //        } else if(advance_to_next_subvoxel(ray, stack[current_stack].origin, size) == -1) {
            //            if(current_stack == 0) {
            //                break;
            //            } else {
            //                current_stack -= 1;
            //            }
            //            break;
            //        }
        } else {
            break;
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