"""Sample Blender Python script referenced by docs and smoke tests.

Invoked by `cli-anything-blender` only when CLI_ANYTHING_BACKEND=system
is set; otherwise the package uses the dry-run backend and just records
the invocation.

Reproduces the minimal "render a cube" example: clear the default
scene, add a cube at the origin, point a camera at it, and render a
single PNG frame to /tmp/cube.png.
"""

import bpy

# Clear the default scene (cube + camera + light are the usual stock).
bpy.ops.object.select_all(action="SELECT")
bpy.ops.object.delete(use_global=False)

# Add a single cube at the origin.
bpy.ops.mesh.primitive_cube_add(location=(0.0, 0.0, 0.0))

# Add a camera pointed at the origin.
bpy.ops.object.camera_add(location=(4.0, -4.0, 3.0), rotation=(1.1, 0.0, 0.785))
bpy.context.scene.camera = bpy.context.object

# Configure and render.
bpy.context.scene.render.image_settings.file_format = "PNG"
bpy.context.scene.render.filepath = "/tmp/cube.png"
bpy.ops.render.render(write_still=True)
