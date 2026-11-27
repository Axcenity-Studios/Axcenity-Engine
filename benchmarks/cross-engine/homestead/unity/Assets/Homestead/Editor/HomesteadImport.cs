using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.HighDefinition;

/// Rebuilds Desert Homestead from the engine's dump (the project's `Dump`
/// folder, written by `dump/dump.sh` and `export_terrain.py`):
///
///   Unity -batchmode -quit -projectPath . -executeMethod HomesteadImport.Import
///   Unity -batchmode -quit -projectPath . -executeMethod HomesteadImport.Build
///
/// The engine's space is right-handed with Y up; Unity's is left-handed.
/// Everything is turned over in X (meshes, places, the terrain, the sun),
/// so the picture is the engine's and not its mirror image.
public static class HomesteadImport
{
    const string Root = "Assets/Homestead";

    static string Dump => Path.Combine(Directory.GetParent(Application.dataPath).FullName, "Dump");

    // scene.json, as JsonUtility reads it.
    [Serializable]
    class SceneFile
    {
        public MaterialDef[] materials;
        public SunDef sun;
    }

    [Serializable]
    class MaterialDef
    {
        public float[] base_color;
        public string base_color_texture;
        public float metalness, roughness;
        public float[] emissive;
        public string emissive_texture;
        public float opacity, alpha_cutoff;
        public string normal_texture;
        public float normal_scale;
        public string mask_texture;
        public bool double_sided;
        public float translucency;
    }

    [Serializable]
    class SunDef
    {
        public float[] direction;
        public float[] color;
        public float intensity;
    }

    [Serializable]
    class TerrainFile
    {
        public TerrainDef[] terrains;
    }

    [Serializable]
    class TerrainDef
    {
        public string name;
        public float[] position, size;
        public int samples;
        public LayerDef[] layers;
    }

    [Serializable]
    class LayerDef
    {
        public string name, textures;
        public float tile_size, normal_strength, height_blend;
        public float[] color;
    }

    class RawMesh
    {
        public ulong[] lods;
        public Vector3[] positions, normals;
        public Vector2[] uvs;
        public Vector4[] tangents;
        public int[] indices;
    }

    [MenuItem("Homestead/Import the dump")]
    public static void Import()
    {
        Directory.CreateDirectory(Root + "/Textures");
        Directory.CreateDirectory(Root + "/Terrain");
        Directory.CreateDirectory(Root + "/Materials");
        CopyTextures();
        Settings();
        var scene = JsonUtility.FromJson<SceneFile>(File.ReadAllText(Path.Combine(Dump, "scene.json")));
        var materials = Materials(scene);
        var catalog = Catalog(materials);
        Scene(scene, catalog);
        AssetDatabase.SaveAssets();
        Debug.Log("HOMESTEAD_IMPORT done");
    }

    /// Copies the dump's images in. HomesteadTextures sets how each is
    /// read as it comes in, by the end of its name.
    static void CopyTextures()
    {
        AssetDatabase.StartAssetEditing();
        try
        {
            foreach (var file in Directory.GetFiles(Path.Combine(Dump, "textures"), "*.png"))
            {
                Copy(file, Root + "/Textures/" + Path.GetFileName(file));
            }
            foreach (var file in Directory.GetFiles(Path.Combine(Dump, "terrain")))
            {
                if (file.EndsWith(".jpg") || file.EndsWith(".png"))
                {
                    Copy(file, Root + "/Terrain/" + Path.GetFileName(file));
                }
            }
            Copy(Path.Combine(Dump, "tour.csv"), Root + "/tour.csv");
        }
        finally
        {
            AssetDatabase.StopAssetEditing();
        }
        AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
    }

    static void Copy(string from, string to)
    {
        if (!File.Exists(to) || File.GetLastWriteTimeUtc(from) > File.GetLastWriteTimeUtc(to))
        {
            File.Copy(from, to, true);
        }
    }

    static Texture2D Texture(string name)
    {
        if (string.IsNullOrEmpty(name))
        {
            return null;
        }
        var texture = AssetDatabase.LoadAssetAtPath<Texture2D>(Root + "/Textures/" + name + ".png");
        if (texture == null)
        {
            Debug.LogWarning("no texture " + name);
        }
        return texture;
    }

    static Color Srgb(float[] c, float alpha = 1f) => new Color(c[0], c[1], c[2], alpha);

    static Material[] Materials(SceneFile scene)
    {
        var lit = Shader.Find("HDRP/Lit");
        var foliage = AssetDatabase.FindAssets("t:DiffusionProfileSettings Foliage")
            .Select(AssetDatabase.GUIDToAssetPath)
            .Select(AssetDatabase.LoadAssetAtPath<DiffusionProfileSettings>)
            .FirstOrDefault(profile => profile != null);
        var materials = new Material[scene.materials.Length];
        for (int i = 0; i < materials.Length; i++)
        {
            var def = scene.materials[i];
            var material = new Material(lit) { name = "m" + i.ToString("000") };
            // Saved first: a diffusion profile can only be given to a saved material.
            AssetDatabase.CreateAsset(material, Root + "/Materials/" + material.name + ".mat");
            material.SetColor("_BaseColor", Srgb(def.base_color, def.opacity));
            var baseMap = Texture(def.base_color_texture);
            if (baseMap != null) material.SetTexture("_BaseColorMap", baseMap);
            var normal = Texture(def.normal_texture);
            if (normal != null)
            {
                material.SetTexture("_NormalMap", normal);
                material.SetFloat("_NormalScale", def.normal_scale);
            }
            var mask = Texture(def.mask_texture);
            if (mask != null)
            {
                // The dump packed the material's own factors into the map.
                material.SetTexture("_MaskMap", mask);
                material.SetFloat("_Metallic", 1f);
                material.SetFloat("_Smoothness", 1f);
            }
            else
            {
                material.SetFloat("_Metallic", def.metalness);
                material.SetFloat("_Smoothness", 1f - def.roughness);
            }
            if (def.emissive[0] + def.emissive[1] + def.emissive[2] > 0f)
            {
                material.SetColor("_EmissiveColor", Srgb(def.emissive).linear);
                var glow = Texture(def.emissive_texture);
                if (glow != null) material.SetTexture("_EmissiveColorMap", glow);
            }
            if (def.alpha_cutoff > 0f)
            {
                material.SetFloat("_AlphaCutoffEnable", 1f);
                material.SetFloat("_AlphaCutoff", def.alpha_cutoff);
            }
            if (def.opacity < 1f)
            {
                material.SetFloat("_SurfaceType", 1f);
            }
            if (def.double_sided)
            {
                material.SetFloat("_DoubleSidedEnable", 1f);
                // Backs lit like fronts.
                material.SetFloat("_DoubleSidedNormalMode", 0f);
            }
            if (def.translucency > 0f && foliage != null)
            {
                // Leaves let light through: HDRP's translucent material.
                material.SetFloat("_MaterialID", 5f);
                material.SetFloat("_Thickness", 1f - def.translucency);
                HDMaterial.SetDiffusionProfile(material, foliage);
            }
            material.enableInstancing = true;
            HDMaterial.ValidateMaterial(material);
            EditorUtility.SetDirty(material);
            materials[i] = material;
        }
        return materials;
    }

    static Dictionary<ulong, RawMesh> ReadMeshes()
    {
        var meshes = new Dictionary<ulong, RawMesh>();
        using var reader = new BinaryReader(File.OpenRead(Path.Combine(Dump, "meshes.bin")));
        int count = reader.ReadInt32();
        for (int m = 0; m < count; m++)
        {
            ulong key = reader.ReadUInt64();
            var mesh = new RawMesh { lods = new ulong[reader.ReadInt32()] };
            for (int i = 0; i < mesh.lods.Length; i++) mesh.lods[i] = reader.ReadUInt64();
            int corners = reader.ReadInt32();
            int indices = reader.ReadInt32();
            int flags = reader.ReadInt32();
            // Turned over in X, which also turns each triangle's winding.
            mesh.positions = new Vector3[corners];
            for (int i = 0; i < corners; i++) mesh.positions[i] = new Vector3(-reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle());
            mesh.normals = new Vector3[corners];
            for (int i = 0; i < corners; i++) mesh.normals[i] = new Vector3(-reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle());
            mesh.uvs = new Vector2[corners];
            if ((flags & 1) != 0)
            {
                // The engine's (0, 0) is the image's top left, Unity's its bottom left.
                for (int i = 0; i < corners; i++) mesh.uvs[i] = new Vector2(reader.ReadSingle(), 1f - reader.ReadSingle());
            }
            if ((flags & 2) != 0)
            {
                mesh.tangents = new Vector4[corners];
                for (int i = 0; i < corners; i++) mesh.tangents[i] = new Vector4(-reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle(), -reader.ReadSingle());
            }
            mesh.indices = new int[indices];
            for (int i = 0; i < indices; i += 3)
            {
                mesh.indices[i] = (int)reader.ReadUInt32();
                mesh.indices[i + 2] = (int)reader.ReadUInt32();
                mesh.indices[i + 1] = (int)reader.ReadUInt32();
            }
            meshes[key] = mesh;
        }
        return meshes;
    }

    /// One mesh of a model's pieces at a level of detail, a submesh a
    /// piece. A piece with fewer levels keeps its last.
    static Mesh Combine(List<(ulong mesh, int material)> pieces, int level, Dictionary<ulong, RawMesh> raw, string name)
    {
        var positions = new List<Vector3>();
        var normals = new List<Vector3>();
        var uvs = new List<Vector2>();
        var tangents = new List<Vector4>();
        var triangles = new List<int[]>();
        bool allTangents = true;
        foreach (var piece in pieces)
        {
            var source = raw[piece.mesh];
            if (level > 0 && source.lods.Length > 0)
            {
                source = raw[source.lods[Mathf.Min(level, source.lods.Length) - 1]];
            }
            int start = positions.Count;
            positions.AddRange(source.positions);
            normals.AddRange(source.normals);
            uvs.AddRange(source.uvs);
            if (source.tangents != null) tangents.AddRange(source.tangents);
            else allTangents = false;
            triangles.Add(source.indices.Select(index => index + start).ToArray());
        }
        var mesh = new Mesh { name = name };
        mesh.indexFormat = positions.Count > 65535 ? IndexFormat.UInt32 : IndexFormat.UInt16;
        mesh.SetVertices(positions);
        mesh.SetNormals(normals);
        mesh.SetUVs(0, uvs);
        mesh.subMeshCount = pieces.Count;
        for (int i = 0; i < triangles.Count; i++) mesh.SetTriangles(triangles[i], i, false);
        if (allTangents) mesh.SetTangents(tangents);
        else mesh.RecalculateTangents();
        mesh.RecalculateBounds();
        mesh.UploadMeshData(false);
        return mesh;
    }

    static HomesteadCatalog Catalog(Material[] materials)
    {
        var raw = ReadMeshes();
        // Items of one id in one place are one model's pieces.
        var order = new List<(ulong id, bool foliage)>();
        var pieces = new Dictionary<(ulong, bool), List<(ulong mesh, int material)>>();
        var first = new Dictionary<(ulong, bool), (float[] transform, float cull, bool casts)>();
        using (var reader = new BinaryReader(File.OpenRead(Path.Combine(Dump, "items.bin"))))
        {
            int count = reader.ReadInt32();
            for (int i = 0; i < count; i++)
            {
                ulong id = reader.ReadUInt64();
                ulong mesh = reader.ReadUInt64();
                int material = reader.ReadInt32();
                bool foliage = reader.ReadInt32() > 0;
                var transform = new float[16];
                for (int k = 0; k < 16; k++) transform[k] = reader.ReadSingle();
                float cull = reader.ReadSingle();
                bool casts = reader.ReadInt32() != 0;
                reader.ReadBytes(16);
                var key = (id, foliage);
                if (!pieces.TryGetValue(key, out var list))
                {
                    list = new List<(ulong, int)>();
                    pieces[key] = list;
                    first[key] = (transform, cull, casts);
                    order.Add(key);
                }
                list.Add((mesh, material));
            }
        }

        var models = new List<HomesteadModel>();
        var modelIds = new Dictionary<string, int>();
        var meshAssets = new List<Mesh>();
        var bytes = new MemoryStream();
        var writer = new BinaryWriter(bytes);
        writer.Write(order.Count);
        foreach (var key in order)
        {
            var list = pieces[key];
            string modelKey = (key.foliage ? "f" : "p") + string.Join(";", list.Select(piece => piece.mesh + ":" + piece.material));
            if (!modelIds.TryGetValue(modelKey, out int model))
            {
                int levels = 1 + list.Max(piece => raw[piece.mesh].lods.Length);
                var lods = new Mesh[levels];
                for (int level = 0; level < levels; level++)
                {
                    lods[level] = Combine(list, level, raw, "model" + models.Count.ToString("000") + "_L" + level);
                    meshAssets.Add(lods[level]);
                }
                model = models.Count;
                models.Add(new HomesteadModel
                {
                    name = "model" + model.ToString("000"),
                    lods = lods,
                    materials = list.Select(piece => materials[piece.material]).ToArray(),
                    foliage = key.foliage,
                });
                modelIds[modelKey] = model;
            }
            // The engine's transform is a place, a turn and a size. Turned
            // over in X: the place's X and the turn's Y and Z change sign.
            var (m, cull, casts) = first[key];
            var c0 = new Vector3(m[0], m[1], m[2]);
            var c1 = new Vector3(m[4], m[5], m[6]);
            var c2 = new Vector3(m[8], m[9], m[10]);
            var scale = new Vector3(c0.magnitude, c1.magnitude, c2.magnitude);
            var rotation = Quaternion.LookRotation(new Vector3(-c2.x, c2.y, c2.z), new Vector3(-c1.x, c1.y, c1.z));
            writer.Write(model);
            writer.Write(-m[12]);
            writer.Write(m[13]);
            writer.Write(m[14]);
            writer.Write(rotation.x);
            writer.Write(rotation.y);
            writer.Write(rotation.z);
            writer.Write(rotation.w);
            writer.Write(scale.x);
            writer.Write(scale.y);
            writer.Write(scale.z);
            writer.Write(cull);
            writer.Write(casts ? 1 : 0);
            writer.Write(0.5f * scale.magnitude);
        }
        writer.Flush();
        File.WriteAllBytes(Root + "/instances.bytes", bytes.ToArray());

        AssetDatabase.DeleteAsset(Root + "/Meshes.asset");
        AssetDatabase.CreateAsset(meshAssets[0], Root + "/Meshes.asset");
        for (int i = 1; i < meshAssets.Count; i++) AssetDatabase.AddObjectToAsset(meshAssets[i], Root + "/Meshes.asset");
        AssetDatabase.SaveAssets();
        AssetDatabase.ImportAsset(Root + "/instances.bytes", ImportAssetOptions.ForceSynchronousImport);

        var catalog = ScriptableObject.CreateInstance<HomesteadCatalog>();
        catalog.models = models.ToArray();
        catalog.instances = AssetDatabase.LoadAssetAtPath<TextAsset>(Root + "/instances.bytes");
        catalog.tour = AssetDatabase.LoadAssetAtPath<TextAsset>(Root + "/tour.csv");
        AssetDatabase.CreateAsset(catalog, Root + "/Catalog.asset");
        Debug.Log($"HOMESTEAD_IMPORT {order.Count} instances of {models.Count} models, {meshAssets.Count} meshes");
        return catalog;
    }

    static void Terrains()
    {
        var file = JsonUtility.FromJson<TerrainFile>(File.ReadAllText(Path.Combine(Dump, "terrain", "terrain.json")));
        var layerAssets = new Dictionary<string, TerrainLayer>();
        var template = new Material(Shader.Find("HDRP/TerrainLit")) { name = "Terrain" };
        // Layers mixed by their painted weights, as the engine mixes them.
        // HDRP's height-based blend keeps only the layer whose height map
        // is highest wherever it has any weight, which loses the painted
        // track and the soil under the plants.
        template.SetFloat("_EnableHeightBlend", 0f);
        template.enableInstancing = true;
        HDMaterial.ValidateMaterial(template);
        AssetDatabase.CreateAsset(template, Root + "/Terrain/Terrain.mat");
        AssetDatabase.SaveAssets();
        // All eight layers in one pass. Set once the material is saved:
        // HDRP resets a new material's keywords as it is imported, and a
        // built player has this shader variant only if a material asks for it.
        template.EnableKeyword("_TERRAIN_8_LAYERS");
        EditorUtility.SetDirty(template);
        AssetDatabase.SaveAssetIfDirty(template);

        foreach (var def in file.terrains)
        {
            int n = def.samples;
            var data = new TerrainData { name = def.name };
            data.heightmapResolution = n;
            data.size = new Vector3(def.size[0], def.size[1], def.size[2]);
            var raw = File.ReadAllBytes(Path.Combine(Dump, "terrain", def.name + "-heights.r16"));
            var heights = new float[n, n];
            for (int z = 0; z < n; z++)
            {
                for (int x = 0; x < n; x++)
                {
                    int at = (z * n + (n - 1 - x)) * 2;
                    heights[z, x] = (raw[at] | raw[at + 1] << 8) / 65535f;
                }
            }
            data.SetHeights(0, 0, heights);
            // Saved before it is painted: the weight textures are kept
            // inside the asset, and ones made before it exists are lost.
            AssetDatabase.CreateAsset(data, Root + "/Terrain/" + def.name + ".asset");

            var layers = new TerrainLayer[def.layers.Length];
            for (int i = 0; i < layers.Length; i++)
            {
                var layer = def.layers[i];
                string key = layer.textures + "|" + layer.tile_size + "|" + layer.normal_strength;
                if (!layerAssets.TryGetValue(key, out layers[i]))
                {
                    var asset = new TerrainLayer
                    {
                        name = layer.name,
                        diffuseTexture = AssetDatabase.LoadAssetAtPath<Texture2D>(Root + "/Terrain/" + layer.textures + "_color.jpg"),
                        normalMapTexture = AssetDatabase.LoadAssetAtPath<Texture2D>(Root + "/Terrain/" + layer.textures + "_normal.jpg"),
                        maskMapTexture = AssetDatabase.LoadAssetAtPath<Texture2D>(Root + "/Terrain/" + layer.textures + "_mask.png"),
                        tileSize = new Vector2(layer.tile_size, layer.tile_size),
                        normalScale = layer.normal_strength,
                        diffuseRemapMax = new Vector4(layer.color[0], layer.color[1], layer.color[2], 1f),
                        metallic = 0f,
                    };
                    AssetDatabase.CreateAsset(asset, Root + "/Terrain/" + layer.name + "-" + layerAssets.Count + ".terrainlayer");
                    layerAssets[key] = asset;
                    layers[i] = asset;
                }
            }
            data.terrainLayers = layers;

            // The weights, a sample to a texel (the engine's are on the
            // height samples, one more each way than Unity allows).
            int res = n - 1;
            data.alphamapResolution = res;
            var paint = File.ReadAllBytes(Path.Combine(Dump, "terrain", def.name + "-paint.rgba8"));
            var weights = new float[res, res, layers.Length];
            for (int z = 0; z < res; z++)
            {
                for (int x = 0; x < res; x++)
                {
                    int sx = Mathf.Clamp(Mathf.RoundToInt((x + 0.5f) / res * (n - 1)), 0, n - 1);
                    int sz = Mathf.Clamp(Mathf.RoundToInt((z + 0.5f) / res * (n - 1)), 0, n - 1);
                    int at = (sz * n + (n - 1 - sx)) * 8;
                    float sum = 0f;
                    for (int l = 0; l < layers.Length; l++) sum += paint[at + l];
                    // Paint claims up to 255 a sample; what it leaves is the
                    // first layer's (terrain/src/materials.rs).
                    float rest = Mathf.Max(0f, 255f - sum);
                    for (int l = 0; l < layers.Length; l++)
                    {
                        weights[z, x, l] = (paint[at + l] + (l == 0 ? rest : 0f)) / (sum + rest);
                    }
                }
            }
            data.SetAlphamaps(0, 0, weights);
            EditorUtility.SetDirty(data);
            AssetDatabase.SaveAssets();

            var go = Terrain.CreateTerrainGameObject(data);
            go.name = def.name;
            go.transform.position = new Vector3(-def.position[0] - def.size[0] * 0.5f, def.position[1], def.position[2] - def.size[2] * 0.5f);
            var terrain = go.GetComponent<Terrain>();
            terrain.materialTemplate = template;
            terrain.drawInstanced = true;
            terrain.shadowCastingMode = ShadowCastingMode.On;
            terrain.drawTreesAndFoliage = false;
        }
    }

    static T Add<T>(VolumeProfile profile) where T : VolumeComponent
    {
        var component = profile.Add<T>(true);
        AssetDatabase.AddObjectToAsset(component, profile);
        return component;
    }

    static void Scene(SceneFile file, HomesteadCatalog catalog)
    {
        var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
        Terrains();

        // The sun: the dump's direction points at it.
        var sunObject = new GameObject("Sun");
        var sun = sunObject.AddComponent<Light>();
        sun.type = LightType.Directional;
        var toSun = new Vector3(-file.sun.direction[0], file.sun.direction[1], file.sun.direction[2]);
        sunObject.transform.rotation = Quaternion.LookRotation(-toSun);
        sun.color = Srgb(file.sun.color);
        sun.shadows = LightShadows.Soft;
        var sunData = sunObject.AddComponent<HDAdditionalLightData>();
        sun.lightUnit = LightUnit.Lux;
        sun.intensity = 100000f;
        sunData.EnableShadows(true);
        sunData.shadowResolution.useOverride = false;
        sunData.shadowResolution.level = 3;
        sunData.useContactShadow.useOverride = true;
        sunData.useContactShadow.@override = true;
        RenderSettings.sun = sun;

        // The look: what the engine's Ultra preset draws for this world.
        var profile = ScriptableObject.CreateInstance<VolumeProfile>();
        AssetDatabase.CreateAsset(profile, Root + "/Look.asset");
        var environment = Add<VisualEnvironment>(profile);
        environment.skyType.value = (int)SkyType.PhysicallyBased;
        environment.skyAmbientMode.value = SkyAmbientMode.Dynamic;
        // The sun, the sky and the fill light are set by eye against the
        // engine's own pictures of the same views (README.md, "The look"):
        // the engine's sky is brighter against its ground than a
        // physically based sky's.
        Add<PhysicallyBasedSky>(profile).exposure.value = 2.15f;
        var indirect = Add<IndirectLightingController>(profile);
        indirect.indirectDiffuseLightingMultiplier.value = 0.3f;
        indirect.reflectionLightingMultiplier.value = 0.3f;
        var fog = Add<Fog>(profile);
        fog.enabled.value = true;
        fog.meanFreePath.value = 4000f;
        fog.baseHeight.value = -40f;
        fog.maximumHeight.value = 400f;
        fog.enableVolumetricFog.value = true;
        fog.depthExtent.value = 300f;
        var clouds = Add<VolumetricClouds>(profile);
        clouds.enable.value = true;
        clouds.cloudPreset = VolumetricClouds.CloudPresets.Sparse;
        clouds.shadows.value = true;
        var exposure = Add<Exposure>(profile);
        exposure.mode.value = ExposureMode.Fixed;
        exposure.fixedExposure.value = 14.55f;
        Add<Tonemapping>(profile).mode.value = TonemappingMode.Neutral;
        var bloom = Add<Bloom>(profile);
        bloom.intensity.value = 0.2f;
        bloom.threshold.value = 1.2f;
        Add<Vignette>(profile).intensity.value = 0.2f;
        var grading = Add<ColorAdjustments>(profile);
        grading.contrast.value = 15f;
        grading.saturation.value = -6f;
        grading.colorFilter.value = new Color(1f, 0.985f, 0.95f);
        var shadows = Add<HDShadowSettings>(profile);
        shadows.maxShadowDistance.value = 300f;
        shadows.cascadeShadowSplitCount.value = 4;
        Add<ContactShadows>(profile).enable.value = true;
        Add<ScreenSpaceAmbientOcclusion>(profile).intensity.value = 1.5f;
        Add<MotionBlur>(profile).intensity.value = 0f;
        Add<DepthOfField>(profile).focusMode.value = DepthOfFieldMode.Off;
        EditorUtility.SetDirty(profile);
        var volumeObject = new GameObject("Look");
        var volume = volumeObject.AddComponent<Volume>();
        volume.isGlobal = true;
        volume.sharedProfile = profile;

        var cameraObject = new GameObject("Camera") { tag = "MainCamera" };
        var camera = cameraObject.AddComponent<Camera>();
        camera.nearClipPlane = 0.1f;
        camera.farClipPlane = 5000f;
        camera.fieldOfView = 36f;
        var cameraData = cameraObject.AddComponent<HDAdditionalCameraData>();
        cameraData.antialiasing = HDAdditionalCameraData.AntialiasingMode.TemporalAntialiasing;
        cameraObject.AddComponent<AudioListener>();

        var worldObject = new GameObject("Homestead");
        var world = worldObject.AddComponent<HomesteadWorld>();
        world.catalog = catalog;
        var bench = worldObject.AddComponent<HomesteadBench>();
        bench.world = world;
        bench.view = camera;
        bench.volume = volume;

        EditorSceneManager.SaveScene(scene, Root + "/Homestead.unity");
        EditorBuildSettings.scenes = new[] { new EditorBuildSettingsScene(Root + "/Homestead.unity", true) };
    }

    /// The pipeline's own switches: instanced drawing from the GPU with
    /// occlusion culling (Unity's way to draw many copies), volumetric
    /// clouds allowed, and frame timings kept for the benchmark.
    static void Settings()
    {
        var asset = (HDRenderPipelineAsset)GraphicsSettings.defaultRenderPipeline;
        var serialized = new SerializedObject(asset);
        void Set(string path, int value)
        {
            var property = serialized.FindProperty("m_RenderPipelineSettings." + path);
            if (property == null) Debug.LogWarning("no HDRP setting " + path);
            else if (property.propertyType == SerializedPropertyType.Boolean) property.boolValue = value != 0;
            else if (property.propertyType == SerializedPropertyType.Float) property.floatValue = value;
            else property.intValue = value;
        }
        Set("supportVolumetricClouds", 1);
        Set("gpuResidentDrawerSettings.mode", 1);
        Set("gpuResidentDrawerSettings.smallMeshScreenPercentage", 0);
        Set("gpuResidentDrawerSettings.enableOcclusionCullingInCameras", 1);
        Set("hdShadowInitParams.directionalShadowFilteringQuality", 2);
        serialized.ApplyModifiedPropertiesWithoutUndo();
        EditorUtility.SetDirty(asset);
        // Keep every BatchRendererGroup shader variant (2), which instanced
        // drawing from the GPU needs in a built player.
        var graphics = new SerializedObject(GraphicsSettings.GetGraphicsSettings());
        var stripping = graphics.FindProperty("m_BrgStripping");
        if (stripping == null) Debug.LogWarning("no m_BrgStripping in the graphics settings");
        else stripping.intValue = 2;
        graphics.ApplyModifiedPropertiesWithoutUndo();

        PlayerSettings.companyName = "Axcenity";
        PlayerSettings.productName = "DesertHomesteadHDRP";
        PlayerSettings.enableFrameTimingStats = true;
        PlayerSettings.runInBackground = true;
        PlayerSettings.fullScreenMode = FullScreenMode.Windowed;
        PlayerSettings.defaultScreenWidth = 1920;
        PlayerSettings.defaultScreenHeight = 1080;
        PlayerSettings.resizableWindow = false;
        QualitySettings.SetQualityLevel(0, true);
        AssetDatabase.SaveAssets();
    }

    [MenuItem("Homestead/Build the player")]
    public static void Build()
    {
        var report = BuildPipeline.BuildPlayer(new BuildPlayerOptions
        {
            scenes = new[] { Root + "/Homestead.unity" },
            locationPathName = "Build/DesertHomesteadHDRP.x86_64",
            target = BuildTarget.StandaloneLinux64,
            options = BuildOptions.None,
        });
        Debug.Log("HOMESTEAD_BUILD " + report.summary.result + ", " + report.summary.totalErrors + " errors");
        if (report.summary.result != BuildResult.Succeeded)
        {
            EditorApplication.Exit(1);
        }
    }

    /// The Windows player with HDRP's ray tracing allowed: DirectX 12 is
    /// the only API HDRP traces rays with (Unity's Vulkan has none), so on
    /// Linux it runs through Proton, whose vkd3d-proton turns DXR into
    /// Vulkan's ray tracing (README.md, "Ray tracing"). The pipeline asset
    /// allows ray tracing and screen-space shadows (which the sun's traced
    /// shadows are drawn into) only for this build and is put back after,
    /// so the Linux player stays as it was measured. Which effects are
    /// traced is chosen per run (HomesteadBench's -hs-set rtshadows, rtao,
    /// rtreflections).
    [MenuItem("Homestead/Build the Windows player (ray tracing)")]
    public static void BuildWindows()
    {
        var asset = (HDRenderPipelineAsset)GraphicsSettings.defaultRenderPipeline;
        string[] switches = { "supportRayTracing", "hdShadowInitParams.supportScreenSpaceShadows", "supportSSR" };
        var before = new Dictionary<string, bool>();
        void Allow(bool on)
        {
            var serialized = new SerializedObject(asset);
            foreach (string path in switches)
            {
                var property = serialized.FindProperty("m_RenderPipelineSettings." + path);
                if (property == null)
                {
                    Debug.LogWarning("no HDRP setting " + path);
                    continue;
                }
                if (on) before[path] = property.boolValue;
                property.boolValue = on || before[path];
            }
            serialized.ApplyModifiedPropertiesWithoutUndo();
            EditorUtility.SetDirty(asset);
            AssetDatabase.SaveAssets();
        }

        var target = BuildTarget.StandaloneWindows64;
        PlayerSettings.SetUseDefaultGraphicsAPIs(target, false);
        PlayerSettings.SetGraphicsAPIs(target, new[] { GraphicsDeviceType.Direct3D12 });
        Allow(true);
        BuildReport report;
        try
        {
            report = BuildPipeline.BuildPlayer(new BuildPlayerOptions
            {
                scenes = new[] { Root + "/Homestead.unity" },
                locationPathName = "Build-Windows/DesertHomesteadHDRP.exe",
                target = target,
                options = BuildOptions.None,
            });
        }
        finally
        {
            Allow(false);
        }
        Debug.Log("HOMESTEAD_BUILD windows " + report.summary.result + ", " + report.summary.totalErrors + " errors");
        if (report.summary.result != BuildResult.Succeeded)
        {
            EditorApplication.Exit(1);
        }
    }
}

/// Reads each image of the dump as what it is, by the end of its name:
/// color maps as sRGB in BC7, normal maps in BC5, mask maps as plain
/// values in BC7 (the formats the engine itself gives the GPU).
public class HomesteadTextures : AssetPostprocessor
{
    void OnPreprocessTexture()
    {
        if (!assetPath.StartsWith("Assets/Homestead/"))
        {
            return;
        }
        var importer = (TextureImporter)assetImporter;
        string name = Path.GetFileNameWithoutExtension(assetPath);
        bool normal = name.EndsWith("_normal");
        bool mask = name.StartsWith("mask_") || name.EndsWith("_mask");
        if (assetPath.Contains("/Textures/") && !mask)
        {
            // The dump names the others by their key alone; the materials
            // say which are normal maps.
            normal = HomesteadTextureUses.Normals.Contains(name);
        }
        importer.textureType = normal ? TextureImporterType.NormalMap : TextureImporterType.Default;
        importer.sRGBTexture = !normal && !mask;
        importer.mipmapEnabled = true;
        importer.maxTextureSize = 8192;
        importer.npotScale = TextureImporterNPOTScale.None;
        importer.anisoLevel = 8;
        importer.alphaIsTransparency = false;
        importer.SetPlatformTextureSettings(new TextureImporterPlatformSettings
        {
            name = "Standalone",
            overridden = true,
            maxTextureSize = 8192,
            format = normal ? TextureImporterFormat.BC5 : TextureImporterFormat.BC7,
            textureCompression = TextureImporterCompression.Compressed,
        });
    }
}

/// Which of the dump's images are normal maps, from its scene.json.
public static class HomesteadTextureUses
{
    static HashSet<string> normals;

    [Serializable]
    class Uses
    {
        public Use[] textures;
    }

    [Serializable]
    class Use
    {
        public string name, usage;
    }

    public static HashSet<string> Normals
    {
        get
        {
            if (normals == null)
            {
                string path = Path.Combine(Directory.GetParent(Application.dataPath).FullName, "Dump", "scene.json");
                var uses = JsonUtility.FromJson<Uses>(File.ReadAllText(path));
                normals = new HashSet<string>(uses.textures.Where(use => use.usage == "normal").Select(use => use.name));
            }
            return normals;
        }
    }
}
