using System;
using System.Collections.Generic;

namespace DeclarationApp.Demo.Avalonia.Models;

/// <summary>Represents a single customs declaration with its associated goods items.</summary>
public class Declaration
{
    public Guid Id { get; set; }
    public string? Number { get; set; }

    /// <summary>Exporter IIN. PII — always trace as Hashed.</summary>
    public string? ExporterIin { get; set; }

    public List<GoodsItem> Goods { get; set; } = new();
    public decimal TotalDuty { get; set; }
}
