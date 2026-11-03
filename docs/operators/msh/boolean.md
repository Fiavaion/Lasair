# Boolean (`msh.boolean`)

MSH · version 1 · cook budget 100 µs

Union, intersection or difference of two closed meshes (the Manifold algorithm, exact predicates). The result is a closed triangle mesh welded by index; float point attributes are interpolated; pieces from A and B are the groups `from_a` and `from_b`. Closed but non-manifold inputs (edges shared by more than two faces) take a slower robust engine; a manifold input that intersects itself is taken as is; open inputs are an error. Normals are not written (add Normals). Cooks on a worker: about 120 ms for 100k triangles a side.

## Inputs

| Name | Type | Notes |
|---|---|---|
| `a` | Geo | optional |
| `b` | Geo | optional |

## Outputs

| Name | Type | Notes |
|---|---|---|
| `out` | Geo |  |

## Parameters

| Name | Label | Type | Default | Range / options | Description |
|---|---|---|---|---|---|
| `operation` | Operation | menu | `"union"` | `union`, `intersect`, `subtract` | union: the space inside A or B; intersect: inside both; subtract: inside A but not B. |

## Platforms

All platforms.

## Examples

- [Mesh Boolean to glTF](../../../examples/mesh-boolean-gltf.lasair/) — `cut`
- [Mesh Modelling](../../../examples/mesh-modelling.lasair/) — `cut`
