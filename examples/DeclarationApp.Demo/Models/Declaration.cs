using System;
using System.Collections.Generic;

namespace DeclarationApp.Demo.Models
{
    /// <summary>
    /// Represents a single customs declaration with its associated goods items
    /// and computed duty total.
    /// </summary>
    public class Declaration
    {
        public Guid Id { get; set; }

        public string Number { get; set; }

        /// <summary>Exporter Individual Identification Number (IIN). PII — always trace as Hashed.</summary>
        public string ExporterIin { get; set; }

        public List<GoodsItem> Goods { get; set; } = new List<GoodsItem>();

        public decimal TotalDuty { get; set; }
    }
}
