---
description: "A light that shines in a cone from just above its parent part."
---

# SpotLight

```luau
Instance.new("SpotLight")
```

A light that shines in a cone from just above its parent part. Anywhere else it needs no part: it shines from its own Position, its Direction turned by its Orientation, and the editor shows it as an icon you can drag.

It also has every member of [Instance](Instance.md).

### Properties

#### Enabled

```luau
SpotLight.Enabled: boolean
```

If false, the light is off.

#### Color

```luau
SpotLight.Color: Color3
```

The light's color.

#### Brightness

```luau
SpotLight.Brightness: number
```

How bright the light is.

#### Range

```luau
SpotLight.Range: number
```

How far the light reaches, in studs.

#### Angle

```luau
SpotLight.Angle: number
```

The full width of the cone, in degrees.

#### Direction

```luau
SpotLight.Direction: Vector3
```

The way the beam points, relative to the parent part (or to its own Orientation when it isn't in a part).

#### Shadows

```luau
SpotLight.Shadows: boolean
```

If true, parts in the beam cast shadows. Only the few spot lights nearest the camera do, as the graphics settings allow.

#### ShadowStart

```luau
SpotLight.ShadowStart: number
```

Studs from the light before anything casts its shadows, every way, in both shadow maps and ray traced shadows. Set it to the size of a fixture the light sits inside, such as a lantern's frame, so the fixture doesn't throw its own shadow across the room. 0 (the default) keeps nothing out but the light's own part, if it's in one.

#### Position

```luau
SpotLight.Position: Vector3
```

Where it is in the world when it isn't inside a part, in studs. Inside a part it works from the part and this isn't used.

#### Orientation

```luau
SpotLight.Orientation: Vector3
```

How it's turned when it isn't inside a part, in degrees, as a part's Orientation. Inside a part it turns with the part.

#### Locked

```luau
SpotLight.Locked: boolean
```

If true, the editor's viewport can't select or change it, and the building tools leave it alone. It has no effect in a running game.

