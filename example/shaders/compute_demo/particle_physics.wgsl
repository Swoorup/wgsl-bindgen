// Advanced particle physics simulation with flocking and gravitational forces
#import global_bindings::{get_time, get_mouse_pos, get_frame_size}

struct Job {
    position: vec3<f32>,
    direction: vec3<f32>,  // velocity vector
    accum: vec3<f32>,      // accumulated force/energy
    depth: u32,            // particle type/age
}

@group(1) @binding(0)
var<storage, read_write> jobs: array<Job>;

struct Params {
    scale: f32,    // gravity strength
    damping: f32,  // velocity damping
}

@group(1) @binding(1)
var<uniform> params: Params;

// Constants for simulation
const DT: f32 = 0.012;               // Slightly faster time step for more responsive motion
const BOUNDARY: f32 = 1.0;           // Full screen boundary (-1 to 1)
const NEIGHBOR_RADIUS: f32 = 0.12;   // Smaller interaction radius to reduce overcrowding
const SEPARATION_RADIUS: f32 = 0.06; // Smaller separation distance for smoother flocking
const MAX_SPEED: f32 = 1.0;          // Increased maximum particle speed for more fluid motion
const MAX_FORCE: f32 = 0.08;         // Increased steering force for better responsiveness
const MOUSE_FORCE_RADIUS: f32 = 0.3; // Smaller mouse radius for more focused interaction
const MOUSE_FORCE_STRENGTH: f32 = 1.2; // Gentler mouse repulsion to reduce bouncing

// Noise function for organic motion
fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(
        mix(hash(i + vec2<f32>(0.0, 0.0)), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

// Limit vector magnitude
fn limit(v: vec3<f32>, max_len: f32) -> vec3<f32> {
    let len = length(v);
    if len > max_len {
        return normalize(v) * max_len;
    }
    return v;
}

// Seek towards a target
fn seek(position: vec3<f32>, to: vec3<f32>, velocity: vec3<f32>) -> vec3<f32> {
    let desired = normalize(to - position) * MAX_SPEED;
    return limit(desired - velocity, MAX_FORCE);
}

// Gravitational attraction/repulsion
fn gravity_force(pos1: vec3<f32>, pos2: vec3<f32>, mass1: f32, mass2: f32, is_repulsive: bool) -> vec3<f32> {
    let diff = pos2 - pos1;
    let dist_sq = max(dot(diff, diff), 0.01); // avoid division by zero
    let dist = sqrt(dist_sq);
    
    let force_magnitude = (mass1 * mass2) / dist_sq * 0.1;
    let direction = normalize(diff);
    
    if is_repulsive {
        return -direction * force_magnitude;
    } else {
        return direction * force_magnitude;
    }
}

// Flocking behaviors
fn separation(index: u32, position: vec3<f32>) -> vec3<f32> {
    var steer = vec3<f32>(0.0);
    var count = 0u;
    
    for (var i = 0u; i < arrayLength(&jobs); i++) {
        if i == index { continue; }
        
        let other_pos = jobs[i].position;
        let dist = distance(position, other_pos);
        
        if dist > 0.0 && dist < SEPARATION_RADIUS {
            let diff = normalize(position - other_pos);
            steer += diff / dist; // Weight by distance
            count++;
        }
    }
    
    if count > 0u {
        steer = steer / f32(count);
        steer = normalize(steer) * MAX_SPEED;
        return limit(steer, MAX_FORCE);
    }
    
    return vec3<f32>(0.0);
}

fn alignment(index: u32, position: vec3<f32>) -> vec3<f32> {
    var neighbor_vel = vec3<f32>(0.0);
    var count = 0u;
    
    for (var i = 0u; i < arrayLength(&jobs); i++) {
        if i == index { continue; }
        
        let other_pos = jobs[i].position;
        let dist = distance(position, other_pos);
        
        if dist > 0.0 && dist < NEIGHBOR_RADIUS {
            neighbor_vel += jobs[i].direction;
            count++;
        }
    }
    
    if count > 0u {
        neighbor_vel = neighbor_vel / f32(count);
        neighbor_vel = normalize(neighbor_vel) * MAX_SPEED;
        return limit(neighbor_vel, MAX_FORCE);
    }
    
    return vec3<f32>(0.0);
}

fn cohesion(index: u32, position: vec3<f32>) -> vec3<f32> {
    var neighbor_pos = vec3<f32>(0.0);
    var count = 0u;
    
    for (var i = 0u; i < arrayLength(&jobs); i++) {
        if i == index { continue; }
        
        let other_pos = jobs[i].position;
        let dist = distance(position, other_pos);
        
        if dist > 0.0 && dist < NEIGHBOR_RADIUS {
            neighbor_pos += other_pos;
            count++;
        }
    }
    
    if count > 0u {
        neighbor_pos = neighbor_pos / f32(count);
        return seek(position, neighbor_pos, jobs[index].direction);
    }
    
    return vec3<f32>(0.0);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index >= arrayLength(&jobs) {
        return;
    }
    
    let t = get_time();
    let gravity_strength = params.scale;
    let damping = params.damping;
    
    let job = &jobs[index];
    let pos = (*job).position;
    let vel = (*job).direction;
    let particle_type = (*job).depth % 4u; // 4 different particle types
    
    // Initialize force accumulator
    var force = vec3<f32>(0.0);
    
    // Add flocking behaviors with different weights based on particle type
    if particle_type == 0u {
        // Flock particles - follow all three rules with balanced weights
        force += separation(index, pos) * 0.8;  // Reduced separation to prevent bouncing
        force += alignment(index, pos) * 1.2;   // Increased alignment for smoother flow
        force += cohesion(index, pos) * 0.6;    // Reduced cohesion to prevent clustering
    } else if particle_type == 1u {
        // Wanderer particles - mostly random movement with light flocking
        let noise_offset = vec2<f32>(pos.x * 0.1 + t * 0.05, pos.y * 0.1 + t * 0.08);
        let noise_force = vec3<f32>(
            noise(noise_offset) - 0.5,
            noise(noise_offset + vec2<f32>(100.0, 0.0)) - 0.5,
            noise(noise_offset + vec2<f32>(0.0, 100.0)) - 0.5
        ) * 0.02;
        force += noise_force;
        force += separation(index, pos) * 0.5;
    } else if particle_type == 2u {
        // Orbiter particles - attracted to center with orbital motion
        let center = vec3<f32>(0.0, 0.0, 0.0);
        let to_center = center - pos;
        let dist_to_center = length(to_center);
        
        if dist_to_center > 0.1 {
            // Gravitational attraction to center
            force += normalize(to_center) * (gravity_strength * 0.5) / (dist_to_center * dist_to_center + 0.1);
            
            // Add tangential force for orbital motion
            let tangent = normalize(cross(to_center, vec3<f32>(0.0, 0.0, 1.0)));
            force += tangent * 0.02;
        }
        
        force += separation(index, pos) * 0.8;
    } else {
        // Repulser particles - repel others and avoid center
        let center = vec3<f32>(0.0, 0.0, 0.0);
        let from_center = pos - center;
        let dist_to_center = length(from_center);
        
        if dist_to_center > 0.1 {
            force += normalize(from_center) * gravity_strength * 0.3;
        }
        
        // Repel nearby particles
        for (var i = 0u; i < arrayLength(&jobs); i++) {
            if i == index { continue; }
            
            let other_pos = jobs[i].position;
            let dist = distance(pos, other_pos);
            
            if dist > 0.0 && dist < NEIGHBOR_RADIUS * 0.5 {
                let repel_force = normalize(pos - other_pos) * (0.1 / (dist + 0.1));
                force += repel_force;
            }
        }
    }
    
    // Add mouse interaction force for all particles
    let mouse_pos = get_mouse_pos();
    let particle_screen_pos = vec2<f32>(pos.x, pos.y);
    let mouse_distance = distance(particle_screen_pos, mouse_pos);
    
    if mouse_distance < MOUSE_FORCE_RADIUS && mouse_distance > 0.001 {
        let mouse_direction = normalize(particle_screen_pos - mouse_pos);
        let mouse_force_strength = MOUSE_FORCE_STRENGTH / (mouse_distance * mouse_distance + 0.01);
        let mouse_force_3d = vec3<f32>(
            mouse_direction.x,
            mouse_direction.y,
            0.0
        ) * mouse_force_strength;
        force += mouse_force_3d;
    }
    
    // Add time-varying forces for more dynamic motion (slowed down)
    let wave_force = vec3<f32>(
        sin(t * 0.6 + pos.y * 0.3) * 0.008,
        cos(t * 0.8 + pos.x * 0.25) * 0.008,
        sin(t * 0.4 + pos.z * 0.2) * 0.004
    ) * gravity_strength;
    force += wave_force;
    
    // Apply forces to velocity
    let new_vel = limit(vel + force, MAX_SPEED);
    
    // Update position
    var new_pos = pos + new_vel * DT;
    
    // Boundary handling with gentle push back (screen space -1 to 1)
    let boundary_softness = 0.9;  // Start applying force before hitting boundary
    let boundary_force = 0.03;    // Much gentler boundary force
    
    // Gentle boundary push-back instead of hard collision
    if abs(new_pos.x) > boundary_softness {
        let push_strength = (abs(new_pos.x) - boundary_softness) / (BOUNDARY - boundary_softness);
        force.x -= sign(new_pos.x) * push_strength * boundary_force;
        if abs(new_pos.x) > BOUNDARY {
            new_pos.x = sign(new_pos.x) * BOUNDARY;
        }
    }
    if abs(new_pos.y) > boundary_softness {
        let push_strength = (abs(new_pos.y) - boundary_softness) / (BOUNDARY - boundary_softness);
        force.y -= sign(new_pos.y) * push_strength * boundary_force;
        if abs(new_pos.y) > BOUNDARY {
            new_pos.y = sign(new_pos.y) * BOUNDARY;
        }
    }
    if abs(new_pos.z) > 0.4 {
        let push_strength = (abs(new_pos.z) - 0.4) / 0.1;
        force.z -= sign(new_pos.z) * push_strength * boundary_force;
        if abs(new_pos.z) > 0.5 {
            new_pos.z = sign(new_pos.z) * 0.5;
        }
    }
    
    // Apply damping
    let damped_vel = new_vel * damping;
    
    // Update particle data
    (*job).position = new_pos;
    (*job).direction = damped_vel;
    (*job).accum += length(force); // Accumulate total force for visualization
    (*job).depth += 1u; // Age the particle
}