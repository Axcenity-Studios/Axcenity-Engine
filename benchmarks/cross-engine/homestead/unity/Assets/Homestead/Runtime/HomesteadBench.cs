using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Text;
using Unity.Profiling;
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.HighDefinition;

/// Runs one benchmark and quits: a still view, or the tour's camera
/// played at a sixtieth of a second a frame, as the engine's `--bench`
/// plays it. Frame time is the wall time between the starts of
/// consecutive frames.
///
///   -hs-view x,y,z,pitch,yaw,fov   a still view (the engine's numbers)
///   -hs-tour <seconds>             the tour from its start, this long
///   -hs-frames <n>                 frames measured for a still (600)
///   -hs-warmup <n>                 frames before measuring (240)
///   -hs-size <w>x<h>               the window's size (1920x1080)
///   -hs-out <file.json>            where the result goes
///   -hs-shot <file.png>            a picture of the last warm-up frame
///   -hs-fullscreen                 exclusive full screen at that size
///   -hs-set name=value,...         changes the look for this run (Set)
public class HomesteadBench : MonoBehaviour
{
    public HomesteadWorld world;
    public Camera view;
    public Volume volume;

    /// How each of the tour's shots is focused, from the project's script:
    /// distance, sharp range, blur in front and behind.
    static readonly (float from, float to, float distance, float range, float near, float far)[] Lenses =
    {
        (11.02f, 20.02f, 9.5f, 4f, 0.5f, 0.9f),
        (20.02f, 30.02f, 16f, 7f, 0.4f, 0.6f),
        (30.02f, 40.02f, 20f, 10f, 0.3f, 0.5f),
    };

    /// The tour's fades (start, end, from, to), from its sequence file.
    static readonly float[,] Fades =
    {
        { 0f, 2.5f, 1f, 0f }, { 19.4f, 20f, 0f, 1f }, { 20f, 20.6f, 1f, 0f }, { 40f, 40.6f, 1f, 0f },
        { 49.4f, 50f, 0f, 1f }, { 50f, 50.6f, 1f, 0f }, { 61.2f, 62f, 0f, 1f },
    };
    const float Letterbox = 0.105f;
    const float LetterboxRamp = 1.5f;

    struct Pose
    {
        public Vector3 position;
        public float pitch, yaw, fov;
    }

    string outPath, shotPath;
    int width = 1920, height = 1080, frames = 600, warmup = 240;
    bool fullscreen, tour;
    float tourSeconds;
    Pose still;
    Pose[] track = Array.Empty<Pose>();
    float tourTime = -1f;
    Texture2D black;
    /// The ray traced effects -hs-set turned on, for the result.
    readonly List<string> traced = new List<string>();

    static string Arg(string name)
    {
        var args = Environment.GetCommandLineArgs();
        int at = Array.IndexOf(args, name);
        return at >= 0 && at + 1 < args.Length ? args[at + 1] : null;
    }

    static float F(string text) => float.Parse(text, CultureInfo.InvariantCulture);

    void Awake()
    {
        outPath = Arg("-hs-out") ?? "homestead-result.json";
        shotPath = Arg("-hs-shot");
        if (Arg("-hs-frames") != null) frames = int.Parse(Arg("-hs-frames"));
        if (Arg("-hs-warmup") != null) warmup = int.Parse(Arg("-hs-warmup"));
        if (Arg("-hs-size") != null)
        {
            var size = Arg("-hs-size").Split('x');
            width = int.Parse(size[0]);
            height = int.Parse(size[1]);
        }
        fullscreen = Array.IndexOf(Environment.GetCommandLineArgs(), "-hs-fullscreen") >= 0;
        // The project's own start view, unless a view is given.
        still = new Pose { position = new Vector3(-5f, 5.26f, 72f), pitch = 0.24f, yaw = -5.55f, fov = 36f };
        if (Arg("-hs-view") != null)
        {
            var v = Arg("-hs-view").Split(',');
            still = new Pose { position = new Vector3(F(v[0]), F(v[1]), F(v[2])), pitch = F(v[3]), yaw = F(v[4]), fov = F(v[5]) };
        }
        if (Arg("-hs-tour") != null)
        {
            tour = true;
            tourSeconds = F(Arg("-hs-tour"));
            track = ReadTrack(world.catalog.tour.text);
            frames = Mathf.Min(Mathf.RoundToInt(tourSeconds * 60f), track.Length);
        }

        QualitySettings.vSyncCount = 0;
        Application.targetFrameRate = -1;
        QualitySettings.lodBias = 1f;
        QualitySettings.maximumLODLevel = 0;
        Application.runInBackground = true;
        Screen.SetResolution(width, height, fullscreen ? FullScreenMode.ExclusiveFullScreen : FullScreenMode.Windowed);
        // Each frame is a sixtieth of a second of game time, whatever it took.
        Time.captureFramerate = 60;
        black = new Texture2D(1, 1);
        black.SetPixel(0, 0, Color.black);
        black.Apply();
    }

    static Pose[] ReadTrack(string csv)
    {
        var poses = new List<Pose>();
        var lines = csv.Split('\n');
        for (int i = 1; i < lines.Length; i++)
        {
            var c = lines[i].Split(',');
            if (c.Length < 9) continue;
            poses.Add(new Pose { position = new Vector3(F(c[2]), F(c[3]), F(c[4])), pitch = F(c[5]), yaw = F(c[6]), fov = F(c[8]) });
        }
        return poses.ToArray();
    }

    /// Places the camera as the engine does: straight ahead is -Z, a
    /// positive pitch looks up and a positive yaw turns toward -X. Unity's
    /// X is the engine's turned over.
    void Place(Pose pose)
    {
        float pitch = Mathf.Clamp(pose.pitch, -89f, 89f) * Mathf.Deg2Rad;
        float yaw = pose.yaw * Mathf.Deg2Rad;
        var look = new Vector3(-Mathf.Sin(yaw) * Mathf.Cos(pitch), Mathf.Sin(pitch), -Mathf.Cos(yaw) * Mathf.Cos(pitch));
        view.transform.SetPositionAndRotation(
            new Vector3(-pose.position.x, pose.position.y, pose.position.z),
            Quaternion.LookRotation(new Vector3(-look.x, look.y, look.z), Vector3.up));
        if (!Mathf.Approximately(view.fieldOfView, pose.fov))
        {
            view.fieldOfView = pose.fov;
            world.BakeLods(pose.fov);
        }
    }

    void Focus(float seconds)
    {
        if (!volume.profile.TryGet(out DepthOfField lens)) return;
        foreach (var l in Lenses)
        {
            if (seconds >= l.from && seconds < l.to)
            {
                lens.focusMode.value = DepthOfFieldMode.Manual;
                lens.nearFocusStart.value = Mathf.Max(0.1f, (l.distance - l.range) * 0.25f);
                lens.nearFocusEnd.value = Mathf.Max(0.2f, l.distance - l.range);
                lens.farFocusStart.value = l.distance + l.range;
                lens.farFocusEnd.value = l.distance + l.range + l.distance * 4f;
                lens.nearMaxBlur = 8f * l.near;
                lens.farMaxBlur = 12f * l.far;
                return;
            }
        }
        lens.focusMode.value = DepthOfFieldMode.Off;
    }

    void OnGUI()
    {
        if (!tour || tourTime < 0f || Event.current.type != EventType.Repaint) return;
        float bars = Letterbox * Mathf.Clamp01(tourTime / LetterboxRamp) * Screen.height;
        GUI.DrawTexture(new Rect(0, 0, Screen.width, bars), black);
        GUI.DrawTexture(new Rect(0, Screen.height - bars, Screen.width, bars), black);
        float fade = 0f;
        for (int i = 0; i < Fades.GetLength(0); i++)
        {
            if (tourTime >= Fades[i, 0] && tourTime <= Fades[i, 1])
            {
                fade = Mathf.Lerp(Fades[i, 2], Fades[i, 3], (tourTime - Fades[i, 0]) / (Fades[i, 1] - Fades[i, 0]));
            }
        }
        if (fade > 0f)
        {
            GUI.color = new Color(1f, 1f, 1f, fade);
            GUI.DrawTexture(new Rect(0, 0, Screen.width, Screen.height), black);
            GUI.color = Color.white;
        }
    }

    /// Changes the look for one run, to find the values the importer then
    /// bakes in and to switch one feature off at a time:
    /// ev (fixed exposure), sun (lux), indirect (sky light on surfaces,
    /// times), tonemap (0 none, 1 neutral, 2 ACES), fogpath, fogbase,
    /// fogmax, volfog (0 or 1), clouds (0 or 1), cloudshadows (0 or 1),
    /// shadows (0 or 1), shadowdistance, ao (intensity, 0 off), contact (0
    /// or 1), contrast, saturation, bloom, skyev (the sky's own exposure,
    /// in stops), aa (0 none, 1 temporal).
    ///
    /// Ray traced effects (the Windows DirectX 12 player only; README.md,
    /// "Ray tracing"), each set to do the work the engine's High does
    /// rather than to HDRP's own High presets, which trace far more:
    /// rtshadows (the sun's rays a pixel, the engine's High is 4; 0 the
    /// shadow maps), rtao (rays a pixel: the engine's High is 8 at half
    /// size, 2 a full-size pixel; to 1.5 units), rtreflections (1: one
    /// mirror ray from surfaces smoother than 0.5, to 800 units, as the
    /// engine's High).
    void Set(string settings)
    {
        // The camera's frame settings, in case the project's defaults
        // leave a traced effect's pass out.
        void Frame(FrameSettingsField field)
        {
            var data = view.GetComponent<HDAdditionalCameraData>();
            data.customRenderingSettings = true;
            data.renderingPathCustomFrameSettingsOverrideMask.mask[(uint)field] = true;
            data.renderingPathCustomFrameSettings.SetEnabled(field, true);
        }
        var profile = volume.profile;
        var sun = RenderSettings.sun != null ? RenderSettings.sun : FindFirstObjectByType<Light>();
        foreach (var pair in settings.Split(','))
        {
            var parts = pair.Split('=');
            float v = F(parts[1]);
            switch (parts[0])
            {
                case "ev": if (profile.TryGet(out Exposure exposure)) exposure.fixedExposure.value = v; break;
                case "sun": sun.intensity = v; break;
                case "indirect":
                    if (!profile.TryGet(out IndirectLightingController indirect)) indirect = profile.Add<IndirectLightingController>(true);
                    indirect.indirectDiffuseLightingMultiplier.value = v;
                    indirect.reflectionLightingMultiplier.value = v;
                    break;
                case "tonemap": if (profile.TryGet(out Tonemapping tone)) tone.mode.value = (TonemappingMode)(int)v; break;
                case "fogpath": if (profile.TryGet(out Fog fog)) fog.meanFreePath.value = v; break;
                case "fogbase": if (profile.TryGet(out Fog fogBase)) fogBase.baseHeight.value = v; break;
                case "fogmax": if (profile.TryGet(out Fog fogMax)) fogMax.maximumHeight.value = v; break;
                case "volfog": if (profile.TryGet(out Fog fogVolume)) fogVolume.enableVolumetricFog.value = v > 0f; break;
                case "clouds": if (profile.TryGet(out VolumetricClouds clouds)) clouds.enable.value = v > 0f; break;
                case "cloudshadows": if (profile.TryGet(out VolumetricClouds cloudShadows)) cloudShadows.shadows.value = v > 0f; break;
                case "shadows": sun.shadows = v > 0f ? LightShadows.Soft : LightShadows.None; break;
                case "shadowdistance": if (profile.TryGet(out HDShadowSettings shadowSettings)) shadowSettings.maxShadowDistance.value = v; break;
                case "ao": if (profile.TryGet(out ScreenSpaceAmbientOcclusion ao)) ao.intensity.value = v; break;
                case "contact": if (profile.TryGet(out ContactShadows contact)) contact.enable.value = v > 0f; break;
                case "contrast": if (profile.TryGet(out ColorAdjustments contrast)) contrast.contrast.value = v; break;
                case "saturation": if (profile.TryGet(out ColorAdjustments saturation)) saturation.saturation.value = v; break;
                case "bloom": if (profile.TryGet(out Bloom bloom)) bloom.intensity.value = v; break;
                case "skyev": if (profile.TryGet(out PhysicallyBasedSky sky)) sky.exposure.value = v; break;
                case "aa":
                    view.GetComponent<HDAdditionalCameraData>().antialiasing = v > 0f
                        ? HDAdditionalCameraData.AntialiasingMode.TemporalAntialiasing
                        : HDAdditionalCameraData.AntialiasingMode.None;
                    break;
                case "rtshadows":
                {
                    var light = sun.GetComponent<HDAdditionalLightData>();
                    light.useScreenSpaceShadows = v > 0f;
                    light.useRayTracedShadows = v > 0f;
                    light.numRayTracingSamples = Mathf.Max(1, (int)v);
                    light.filterTracedShadow = true;
                    if (v > 0f)
                    {
                        Frame(FrameSettingsField.RayTracing);
                        Frame(FrameSettingsField.ScreenSpaceShadows);
                        traced.Add("shadows " + (int)v);
                    }
                    break;
                }
                case "rtao":
                    if (profile.TryGet(out ScreenSpaceAmbientOcclusion rtao) && v > 0f)
                    {
                        rtao.rayTracing.value = true;
                        // Custom values, not a quality level's.
                        rtao.quality.value = ScalableSettingLevelParameter.LevelCount;
                        rtao.rayLength = 1.5f;
                        rtao.sampleCount = (int)v;
                        rtao.denoise = true;
                        Frame(FrameSettingsField.RayTracing);
                        Frame(FrameSettingsField.SSAO);
                        traced.Add($"ao {rtao.sampleCount} rays to {rtao.rayLength}");
                    }
                    break;
                case "rtreflections":
                    if (v > 0f)
                    {
                        if (!profile.TryGet(out ScreenSpaceReflection reflection)) reflection = profile.Add<ScreenSpaceReflection>(true);
                        reflection.enabled.value = true;
                        reflection.tracing.value = RayCastingMode.RayTracing;
                        reflection.mode.value = UnityEngine.Rendering.HighDefinition.RayTracingMode.Performance;
                        reflection.quality.value = ScalableSettingLevelParameter.LevelCount;
                        reflection.minSmoothness = 0.5f;
                        reflection.smoothnessFadeStart = 0.5f;
                        reflection.rayLength = 800f;
                        reflection.fullResolution = true;
                        reflection.denoise = true;
                        Frame(FrameSettingsField.RayTracing);
                        Frame(FrameSettingsField.SSR);
                        traced.Add($"reflections from smoothness {reflection.minSmoothness} to {reflection.rayLength}");
                    }
                    break;
                default: UnityEngine.Debug.LogWarning("-hs-set: no setting " + parts[0]); break;
            }
        }
    }

    IEnumerator Start()
    {
        if (Arg("-hs-set") != null) Set(Arg("-hs-set"));
        var load = Stopwatch.StartNew();
        world.Spawn();
        view.fieldOfView = 1f;
        Place(tour ? track[0] : still);
        double loadMs = load.Elapsed.TotalMilliseconds;

        for (int i = 0; i < warmup; i++)
        {
            if (shotPath != null && i == warmup - 2) ScreenCapture.CaptureScreenshot(shotPath);
            yield return null;
        }

        var recorders = new Dictionary<string, ProfilerRecorder>
        {
            { "draw_calls", ProfilerRecorder.StartNew(ProfilerCategory.Render, "Draw Calls Count") },
            { "batches", ProfilerRecorder.StartNew(ProfilerCategory.Render, "Batches Count") },
            { "set_pass_calls", ProfilerRecorder.StartNew(ProfilerCategory.Render, "SetPass Calls Count") },
            { "triangles", ProfilerRecorder.StartNew(ProfilerCategory.Render, "Triangles Count") },
            { "vertices", ProfilerRecorder.StartNew(ProfilerCategory.Render, "Vertices Count") },
            { "shadow_casters", ProfilerRecorder.StartNew(ProfilerCategory.Render, "Shadow Casters Count") },
            { "system_memory_bytes", ProfilerRecorder.StartNew(ProfilerCategory.Memory, "System Used Memory") },
            { "gfx_memory_bytes", ProfilerRecorder.StartNew(ProfilerCategory.Memory, "Gfx Used Memory") },
        };
        var sums = new Dictionary<string, double>();
        foreach (var name in recorders.Keys) sums[name] = 0;

        var times = new double[frames];
        var timings = new FrameTiming[1];
        double cpuMain = 0, cpuRender = 0, cpuFrame = 0, gpu = 0;
        int timed = 0;
        double startedUnix = (DateTime.UtcNow - new DateTime(1970, 1, 1)).TotalSeconds;
        var clock = Stopwatch.StartNew();
        long last = clock.ElapsedTicks;
        for (int i = 0; i < frames; i++)
        {
            if (tour)
            {
                tourTime = i / 60f;
                Place(track[i]);
                Focus(tourTime);
            }
            yield return null;
            long now = clock.ElapsedTicks;
            times[i] = (now - last) * 1000.0 / Stopwatch.Frequency;
            last = now;
            FrameTimingManager.CaptureFrameTimings();
            if (FrameTimingManager.GetLatestTimings(1, timings) > 0 && timings[0].cpuFrameTime > 0)
            {
                cpuMain += timings[0].cpuMainThreadFrameTime;
                cpuRender += timings[0].cpuRenderThreadFrameTime;
                cpuFrame += timings[0].cpuFrameTime;
                gpu += timings[0].gpuFrameTime;
                timed++;
            }
            foreach (var pair in recorders) sums[pair.Key] += pair.Value.LastValue;
        }
        double seconds = clock.Elapsed.TotalSeconds;

        var sorted = (double[])times.Clone();
        Array.Sort(sorted);
        double total = 0;
        foreach (double t in times) total += t;
        // The 1% and 0.1% lows: the average rate of the slowest frames.
        double Low(double share)
        {
            int n = Mathf.Max(1, Mathf.RoundToInt((float)(frames * share)));
            double sum = 0;
            for (int i = frames - n; i < frames; i++) sum += sorted[i];
            return 1000.0 * n / sum;
        }
        var json = new StringBuilder();
        var inv = CultureInfo.InvariantCulture;
        json.Append("{\n");
        json.AppendFormat(inv, " \"engine\": \"Unity\", \"version\": \"{0}\", \"pipeline\": \"HDRP\",\n", Application.unityVersion);
        json.AppendFormat(inv, " \"graphics_api\": \"{0}\", \"gpu\": \"{1}\",\n", SystemInfo.graphicsDeviceType, SystemInfo.graphicsDeviceName);
        json.AppendFormat(inv, " \"ray_tracing_supported\": {0},\n", SystemInfo.supportsRayTracing ? "true" : "false");
        var pipeline = RenderPipelineManager.currentPipeline as HDRenderPipeline;
        json.AppendFormat(inv, " \"hdrp_ray_tracing\": {0}, \"ray_traced\": \"{1}\",\n",
            pipeline != null && pipeline.rayTracingSupported ? "true" : "false", string.Join("; ", traced));
        json.AppendFormat(inv, " \"drawn_at\": [{0}, {1}], \"fullscreen\": {2},\n", Screen.width, Screen.height, Screen.fullScreen ? "true" : "false");
        json.AppendFormat(inv, " \"instances\": {0}, \"load_ms\": {1:F0}, \"set\": \"{2}\",\n", world.Count, loadMs, Arg("-hs-set") ?? "");
        json.AppendFormat(inv, " \"frames\": {0}, \"seconds\": {1:F3}, \"measure_start_unix\": {2:F3},\n", frames, seconds, startedUnix);
        json.AppendFormat(inv, " \"fps_average\": {0:F2}, \"mean_ms\": {1:F3}, \"median_ms\": {2:F3}, \"p95_ms\": {3:F3}, \"p99_ms\": {4:F3}, \"worst_ms\": {5:F3},\n",
            1000.0 * frames / total, total / frames, sorted[frames / 2], sorted[(int)(frames * 0.95)], sorted[(int)(frames * 0.99)], sorted[frames - 1]);
        json.AppendFormat(inv, " \"low_1_percent_fps\": {0:F2}, \"low_01_percent_fps\": {1:F2},\n", Low(0.01), Low(0.001));
        if (timed > 0)
        {
            json.AppendFormat(inv, " \"cpu_main_thread_ms\": {0:F3}, \"cpu_render_thread_ms\": {1:F3}, \"cpu_frame_ms\": {2:F3}, \"gpu_ms\": {3:F3},\n",
                cpuMain / timed, cpuRender / timed, cpuFrame / timed, gpu / timed);
        }
        foreach (var pair in sums)
        {
            json.AppendFormat(inv, " \"{0}\": {1:F0},\n", pair.Key, pair.Value / frames);
        }
        json.Append(" \"frame_ms\": [");
        for (int i = 0; i < frames; i++)
        {
            if (i > 0) json.Append(',');
            json.Append(times[i].ToString("F3", inv));
        }
        json.Append("]\n}\n");
        File.WriteAllText(outPath, json.ToString());
        UnityEngine.Debug.Log("HOMESTEAD_RESULT written to " + outPath);
        foreach (var recorder in recorders.Values) recorder.Dispose();
        Application.Quit();
    }
}
