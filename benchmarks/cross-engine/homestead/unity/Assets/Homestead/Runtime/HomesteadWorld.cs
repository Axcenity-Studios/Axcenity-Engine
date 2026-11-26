using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;
using UnityEngine.Rendering;

/// Spawns every instance of the catalog and gives each the engine's own
/// drawing rules as a LODGroup: a simpler mesh below 30% and 12% of the
/// screen's height (its bounding sphere's), no shadow past 35% of its cull
/// distance, and nothing past the cull distance.
public class HomesteadWorld : MonoBehaviour
{
    public HomesteadCatalog catalog;

    /// The engine's LOD_SCREEN_SIZES and SHADOW_REACH (render/src/renderer/lod.rs).
    static readonly float[] LodScreenSizes = { 0.3f, 0.12f };
    const float ShadowReach = 0.35f;

    struct Spawned
    {
        public LODGroup group;
        public int model;
        public float radius;
        public float cullDistance;
        public bool castsShadow;
        /// By level, then with a shadow (0) and without (1); null where
        /// the model has no such child.
        public Renderer[] renderers;
    }

    Spawned[] spawned = Array.Empty<Spawned>();
    public int Count => spawned.Length;

    public void Spawn()
    {
        var models = catalog.models;
        // One hidden template per model, copied for each instance.
        var templates = new GameObject[models.Length];
        var holder = new GameObject("Templates");
        holder.SetActive(false);
        for (int m = 0; m < models.Length; m++)
        {
            var model = models[m];
            var root = new GameObject(model.name);
            root.transform.SetParent(holder.transform, false);
            root.AddComponent<LODGroup>();
            for (int level = 0; level < model.lods.Length; level++)
            {
                for (int variant = 0; variant < (model.foliage ? 2 : 1); variant++)
                {
                    var child = new GameObject(variant == 0 ? "L" + level : "L" + level + " no shadow");
                    child.transform.SetParent(root.transform, false);
                    child.AddComponent<MeshFilter>().sharedMesh = model.lods[level];
                    var renderer = child.AddComponent<MeshRenderer>();
                    renderer.sharedMaterials = model.materials;
                    renderer.shadowCastingMode = variant == 0 ? ShadowCastingMode.On : ShadowCastingMode.Off;
                    renderer.motionVectorGenerationMode = MotionVectorGenerationMode.Camera;
                    renderer.lightProbeUsage = LightProbeUsage.Off;
                    renderer.reflectionProbeUsage = ReflectionProbeUsage.BlendProbes;
                    child.SetActive(false);
                }
            }
            templates[m] = root;
        }

        var parent = new GameObject("Instances").transform;
        using var reader = new BinaryReader(new MemoryStream(catalog.instances.bytes));
        int count = reader.ReadInt32();
        spawned = new Spawned[count];
        for (int i = 0; i < count; i++)
        {
            int m = reader.ReadInt32();
            var position = new Vector3(reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle());
            var rotation = new Quaternion(reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle());
            var scale = new Vector3(reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle());
            float cull = reader.ReadSingle();
            bool casts = reader.ReadInt32() != 0;
            float radius = reader.ReadSingle();

            var model = models[m];
            var instance = Instantiate(templates[m], position, rotation, parent);
            instance.transform.localScale = scale;
            int variants = model.foliage ? 2 : 1;
            var renderers = new Renderer[model.lods.Length * 2];
            for (int level = 0; level < model.lods.Length; level++)
            {
                for (int variant = 0; variant < variants; variant++)
                {
                    renderers[level * 2 + variant] = instance.transform.GetChild(level * variants + variant).GetComponent<Renderer>();
                }
            }
            var group = instance.GetComponent<LODGroup>();
            // The engine's sphere: the item's middle, and half its box's diagonal.
            group.localReferencePoint = Vector3.zero;
            group.size = 2f * radius / Mathf.Max(scale.x, scale.y, scale.z);
            group.fadeMode = LODFadeMode.CrossFade;
            group.animateCrossFading = true;
            spawned[i] = new Spawned
            {
                group = group, model = m, radius = radius, cullDistance = cull,
                castsShadow = casts, renderers = renderers,
            };
        }
        Destroy(holder);
    }

    /// Sets every instance's levels for a camera of this field of view.
    /// The engine culls and drops shadows by distance and Unity by size on
    /// screen, so the sizes are worked out for the field of view in use.
    public void BakeLods(float fieldOfView)
    {
        float tanHalf = Mathf.Tan(fieldOfView * 0.5f * Mathf.Deg2Rad);
        var edges = new List<float>(4);
        var lods = new List<LOD>(5);
        var used = new bool[6];
        foreach (var s in spawned)
        {
            int levels = catalog.models[s.model].lods.Length;
            bool culled = s.cullDistance > 0f;
            float cullSize = culled ? s.radius / (s.cullDistance * tanHalf) : 0f;
            float shadowSize = culled ? cullSize / ShadowReach : 0f;
            edges.Clear();
            for (int level = 1; level < levels; level++)
            {
                edges.Add(LodScreenSizes[level - 1]);
            }
            if (culled && s.castsShadow)
            {
                edges.Add(shadowSize);
            }
            edges.Add(cullSize);
            edges.Sort((a, b) => b.CompareTo(a));
            lods.Clear();
            Array.Clear(used, 0, used.Length);
            float above = float.PositiveInfinity;
            foreach (float edge in edges)
            {
                // Edges under the cull size are never reached, and two the
                // same make no band.
                if (edge < cullSize || edge >= above)
                {
                    continue;
                }
                // What is drawn between this edge and the one above it.
                float size = float.IsInfinity(above) ? edge + 1f : (edge + above) * 0.5f;
                int level = 0;
                while (level < levels - 1 && size < LodScreenSizes[level])
                {
                    level++;
                }
                bool shadow = s.castsShadow && (!culled || size >= shadowSize);
                int index = level * 2 + (shadow ? 0 : 1);
                if (s.renderers[index] == null)
                {
                    index = level * 2;
                    s.renderers[index].shadowCastingMode = shadow ? ShadowCastingMode.On : ShadowCastingMode.Off;
                }
                used[index] = true;
                float height = Mathf.Clamp(edge, 0.00001f, 0.99f - lods.Count * 0.0001f);
                lods.Add(new LOD(height, new[] { s.renderers[index] }));
                above = edge;
            }
            for (int index = 0; index < s.renderers.Length; index++)
            {
                if (s.renderers[index] != null && s.renderers[index].gameObject.activeSelf != used[index])
                {
                    s.renderers[index].gameObject.SetActive(used[index]);
                }
            }
            s.group.SetLODs(lods.ToArray());
        }
    }
}
