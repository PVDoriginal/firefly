# Examples
Various examples for using Firefly. 

More examples are coming soon. In the meantime, consider checking the [crates.io documentation](https://crates.io/crates/bevy_firefly/) to learn about implementing certain advanced features. 

## Simple 
Simple example of integrating firefly into your bevy app and spawning a light and some round occluders. You can move the light using the arrow keys.

<img width="791" height="460" alt="image" src="https://github.com/user-attachments/assets/7e0b87d4-0b61-4687-926b-341591a20a38" />

## Shapes
Example showcasing a light interacting with occluders of various shapes. You can click around to move the light. 

https://github.com/user-attachments/assets/53649d74-b6c2-49b1-a3ba-29032080865a

## Crates
Example showcasing normal maps and occlusion z-sorting. You can click and drag on objects to move them around. 

https://github.com/user-attachments/assets/04f9870e-2064-4724-b3f7-e9f5c7791b8c

## Flashlight
Example showcasing custom LightAngles and rotating occluders. 
<img width="980" height="593" alt="image" src="https://github.com/user-attachments/assets/1c607b15-0fdd-4571-a482-300dbe395f3d" />


## Blending
Example showcasing how two lights blend. 
<img width="1089" height="715" alt="image" src="https://github.com/user-attachments/assets/581976ee-7136-44dc-a528-ac84c9d2a99d" />

## Noise
Example teaching users to grab the LightMap and modify its value in a custom render pass. 
<img width="830" height="573" alt="image" src="https://github.com/user-attachments/assets/80855e90-fb05-4fa0-9b45-fc0802887f2d" />


## Stress
A stress test for firefly. It spawns a large amount of lights and occluders. You can press the left and right arrows to zoom in an out. 
This shouldn't be used as an example on using firefly, it's simply used to test the performance impact of new features and optimizations.

Includes a performance meter. 

<img width="1291" height="769" alt="image" src="https://github.com/user-attachments/assets/97804e4a-a9de-43a4-ace6-42ac03730010" />
