"""Reading and rewriting glTF binary files without Blender: what
`default_model.py` needs to make the default humanoid's model out of the
base's own files, bone for bone (an import and export through Blender
would give the bones other axes).

A `Glb` is the file's JSON and its one binary buffer. `strip_targets`
drops shapes, `add_meshes` brings another file's skinned meshes onto this
file's skeleton, and `write` leaves out everything nothing refers to any
more.
"""

import json
import struct

JSON_CHUNK, BIN_CHUNK = 0x4E4F534A, 0x004E4942


class Glb:
    def __init__(self, path):
        with open(path, "rb") as file:
            data = file.read()
        magic, version, _ = struct.unpack_from("<4sII", data, 0)
        if magic != b"glTF" or version != 2:
            raise ValueError(f"{path} is not a glTF 2 binary file")
        length, kind = struct.unpack_from("<II", data, 12)
        if kind != JSON_CHUNK:
            raise ValueError(f"{path}: the first chunk is not JSON")
        self.json = json.loads(data[20:20 + length])
        at = 20 + length
        self.bin = b""
        if at < len(data):
            length, kind = struct.unpack_from("<II", data, at)
            if kind == BIN_CHUNK:
                self.bin = data[at + 8:at + 8 + length]
        if len(self.json.get("buffers", [])) > 1:
            raise ValueError(f"{path} has more than one buffer")

    def view(self, index):
        """A buffer view's bytes."""
        view = self.json["bufferViews"][index]
        start = view.get("byteOffset", 0)
        return self.bin[start:start + view["byteLength"]]

    def node_named(self, name):
        return next(i for i, node in enumerate(self.json["nodes"]) if node.get("name") == name)

    def joint_names(self, skin=0):
        return [self.json["nodes"][j].get("name") for j in self.json["skins"][skin]["joints"]]

    def mesh_nodes(self):
        return [i for i, node in enumerate(self.json["nodes"]) if "mesh" in node]

    def strip_targets(self, keep=()):
        """Drops every shape whose name isn't in `keep` from every mesh."""
        for mesh in self.json["meshes"]:
            names = (mesh.get("extras") or {}).get("targetNames") or []
            kept = [i for i, name in enumerate(names) if name in keep]
            for primitive in mesh["primitives"]:
                targets = primitive.get("targets")
                if targets is None:
                    continue
                if kept:
                    primitive["targets"] = [targets[i] for i in kept]
                else:
                    del primitive["targets"]
            if kept:
                mesh["extras"]["targetNames"] = [names[i] for i in kept]
                mesh["weights"] = [0.0] * len(kept)
            else:
                mesh.pop("weights", None)
                if "extras" in mesh:
                    mesh["extras"].pop("targetNames", None)
                    if not mesh["extras"]:
                        del mesh["extras"]

    def remove_mesh_nodes(self, names):
        """Takes the mesh nodes called `names` out of the scene."""
        gone = {i for i in self.mesh_nodes() if self.json["nodes"][i].get("name") in names}
        for node in self.json["nodes"]:
            if "children" in node:
                node["children"] = [c for c in node["children"] if c not in gone]
        for scene in self.json["scenes"]:
            scene["nodes"] = [n for n in scene["nodes"] if n not in gone]
        for i in gone:
            self.json["nodes"][i] = {"name": self.json["nodes"][i].get("name", "")}

    def add_meshes(self, other, under):
        """Brings `other`'s skinned mesh nodes into this file as children
        of the node `under`, on this file's skeleton: the two skins must
        name the same joints in the same order."""
        if other.joint_names() != self.joint_names():
            raise ValueError("the two files' skeletons differ")
        j, o = self.json, other.json
        base = len(self.bin)
        self.bin += other.bin + b"\0" * (-len(other.bin) % 4)
        views = len(j["bufferViews"])
        for view in o.get("bufferViews", []):
            view = dict(view)
            view["byteOffset"] = view.get("byteOffset", 0) + base
            j["bufferViews"].append(view)
        accessors = len(j["accessors"])
        for accessor in o.get("accessors", []):
            accessor = json.loads(json.dumps(accessor))
            if "bufferView" in accessor:
                accessor["bufferView"] += views
            sparse = accessor.get("sparse")
            if sparse:
                sparse["indices"]["bufferView"] += views
                sparse["values"]["bufferView"] += views
            j["accessors"].append(accessor)
        images = len(j.setdefault("images", []))
        for image in o.get("images", []):
            image = dict(image)
            if "bufferView" in image:
                image["bufferView"] += views
            j["images"].append(image)
        samplers = len(j.setdefault("samplers", []))
        j["samplers"].extend(o.get("samplers", []))
        textures = len(j.setdefault("textures", []))
        for texture in o.get("textures", []):
            texture = dict(texture)
            if "source" in texture:
                texture["source"] += images
            if "sampler" in texture:
                texture["sampler"] += samplers
            j["textures"].append(texture)

        def retexture(value):
            if isinstance(value, dict):
                return {k: (v + textures if k == "index" and isinstance(v, int) else retexture(v))
                        for k, v in value.items()}
            if isinstance(value, list):
                return [retexture(v) for v in value]
            return value

        materials = len(j.setdefault("materials", []))
        j["materials"].extend(retexture(m) for m in o.get("materials", []))
        meshes = len(j["meshes"])
        for mesh in o.get("meshes", []):
            mesh = json.loads(json.dumps(mesh))
            for primitive in mesh["primitives"]:
                primitive["attributes"] = {k: v + accessors for k, v in primitive["attributes"].items()}
                if "indices" in primitive:
                    primitive["indices"] += accessors
                if "material" in primitive:
                    primitive["material"] += materials
                if "targets" in primitive:
                    primitive["targets"] = [{k: v + accessors for k, v in t.items()} for t in primitive["targets"]]
            j["meshes"].append(mesh)
        parent = j["nodes"][under]
        for index in other.mesh_nodes():
            node = dict(o["nodes"][index])
            node["mesh"] += meshes
            node["skin"] = 0
            node.pop("children", None)
            j["nodes"].append(node)
            parent.setdefault("children", []).append(len(j["nodes"]) - 1)

    def _used_accessors(self):
        j = self.json
        used = set()
        live_meshes = {node["mesh"] for node in j["nodes"] if "mesh" in node}
        for index in live_meshes:
            for primitive in j["meshes"][index]["primitives"]:
                used.update(primitive["attributes"].values())
                if "indices" in primitive:
                    used.add(primitive["indices"])
                for target in primitive.get("targets", []):
                    used.update(target.values())
        for skin in j.get("skins", []):
            if "inverseBindMatrices" in skin:
                used.add(skin["inverseBindMatrices"])
        for animation in j.get("animations", []):
            for sampler in animation["samplers"]:
                used.update((sampler["input"], sampler["output"]))
        return used, live_meshes

    def write(self, path):
        """Writes the file with only the meshes, materials, textures,
        images, accessors and bytes something still refers to."""
        j = self.json
        accessors, meshes = self._used_accessors()
        mesh_map = {old: new for new, old in enumerate(sorted(meshes))}
        materials = sorted({p["material"] for m in meshes for p in j["meshes"][m]["primitives"] if "material" in p})
        material_map = {old: new for new, old in enumerate(materials)}

        def texture_indices(value, found):
            if isinstance(value, dict):
                for k, v in value.items():
                    if k == "index" and isinstance(v, int):
                        found.add(v)
                    else:
                        texture_indices(v, found)
            elif isinstance(value, list):
                for v in value:
                    texture_indices(v, found)
            return found

        textures = sorted(texture_indices([j["materials"][m] for m in materials], set())) if materials else []
        texture_map = {old: new for new, old in enumerate(textures)}
        images = sorted({j["textures"][t]["source"] for t in textures if "source" in j["textures"][t]})
        image_map = {old: new for new, old in enumerate(images)}
        accessor_map = {old: new for new, old in enumerate(sorted(accessors))}

        out, new_views, view_map = bytearray(), [], {}

        def keep_view(index):
            if index not in view_map:
                view = dict(j["bufferViews"][index])
                data = self.view(index)
                out.extend(b"\0" * (-len(out) % 4))
                view["byteOffset"] = len(out)
                view["buffer"] = 0
                out.extend(data)
                view_map[index] = len(new_views)
                new_views.append(view)
            return view_map[index]

        new_accessors = []
        for old in sorted(accessors):
            accessor = json.loads(json.dumps(j["accessors"][old]))
            if "bufferView" in accessor:
                accessor["bufferView"] = keep_view(accessor["bufferView"])
            sparse = accessor.get("sparse")
            if sparse:
                sparse["indices"]["bufferView"] = keep_view(sparse["indices"]["bufferView"])
                sparse["values"]["bufferView"] = keep_view(sparse["values"]["bufferView"])
            new_accessors.append(accessor)
        new_images = []
        for old in images:
            image = dict(j["images"][old])
            if "bufferView" in image:
                image["bufferView"] = keep_view(image["bufferView"])
            new_images.append(image)
        new_textures = []
        for old in textures:
            texture = dict(j["textures"][old])
            if "source" in texture:
                texture["source"] = image_map[texture["source"]]
            new_textures.append(texture)

        def retexture(value):
            if isinstance(value, dict):
                return {k: (texture_map[v] if k == "index" and isinstance(v, int) else retexture(v))
                        for k, v in value.items()}
            if isinstance(value, list):
                return [retexture(v) for v in value]
            return value

        new_meshes = []
        for old in sorted(meshes):
            mesh = json.loads(json.dumps(j["meshes"][old]))
            for primitive in mesh["primitives"]:
                primitive["attributes"] = {k: accessor_map[v] for k, v in primitive["attributes"].items()}
                if "indices" in primitive:
                    primitive["indices"] = accessor_map[primitive["indices"]]
                if "material" in primitive:
                    primitive["material"] = material_map[primitive["material"]]
                if "targets" in primitive:
                    primitive["targets"] = [{k: accessor_map[v] for k, v in t.items()} for t in primitive["targets"]]
            new_meshes.append(mesh)
        made = dict(j)
        made["nodes"] = []
        for node in j["nodes"]:
            node = dict(node)
            if "mesh" in node:
                node["mesh"] = mesh_map[node["mesh"]]
            made["nodes"].append(node)
        made["skins"] = []
        for skin in j.get("skins", []):
            skin = dict(skin)
            if "inverseBindMatrices" in skin:
                skin["inverseBindMatrices"] = accessor_map[skin["inverseBindMatrices"]]
            made["skins"].append(skin)
        if "animations" in j:
            made["animations"] = []
            for animation in j["animations"]:
                animation = json.loads(json.dumps(animation))
                for sampler in animation["samplers"]:
                    sampler["input"] = accessor_map[sampler["input"]]
                    sampler["output"] = accessor_map[sampler["output"]]
                made["animations"].append(animation)
        made["meshes"] = new_meshes
        made["accessors"] = new_accessors
        made["bufferViews"] = new_views
        made["materials"] = [retexture(j["materials"][m]) for m in materials]
        made["textures"] = new_textures
        made["images"] = new_images
        for key in ("materials", "textures", "images", "samplers"):
            if not made.get(key):
                made.pop(key, None)
        out.extend(b"\0" * (-len(out) % 4))
        made["buffers"] = [{"byteLength": len(out)}]
        text = json.dumps(made, separators=(",", ":")).encode()
        text += b" " * (-len(text) % 4)
        total = 12 + 8 + len(text) + 8 + len(out)
        with open(path, "wb") as file:
            file.write(struct.pack("<4sII", b"glTF", 2, total))
            file.write(struct.pack("<II", len(text), JSON_CHUNK))
            file.write(text)
            file.write(struct.pack("<II", len(out), BIN_CHUNK))
            file.write(out)
        return total
