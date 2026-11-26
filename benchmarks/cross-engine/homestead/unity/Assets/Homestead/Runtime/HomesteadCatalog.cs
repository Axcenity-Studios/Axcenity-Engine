using System;
using UnityEngine;

/// One thing the engine draws as several items in one place (a model's
/// primitives): a mesh per level of detail, each with a submesh per
/// primitive, and the primitives' materials.
[Serializable]
public class HomesteadModel
{
    public string name;
    public Mesh[] lods;
    public Material[] materials;
    /// Painted foliage, which the engine culls by distance and stops
    /// casting shadows from at 35% of that distance.
    public bool foliage;
}

/// Everything the importer made from the engine's dump, for the scene's
/// HomesteadWorld to spawn.
public class HomesteadCatalog : ScriptableObject
{
    public HomesteadModel[] models;
    /// The instances, as HomesteadWorld reads them.
    public TextAsset instances;
    /// The tour's camera at each sixtieth of a second (the engine's
    /// tour.csv, in the engine's own space).
    public TextAsset tour;
}
