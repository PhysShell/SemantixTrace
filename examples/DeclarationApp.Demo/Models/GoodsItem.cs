using System;

namespace DeclarationApp.Demo.Models
{
    /// <summary>
    /// A single line item in a customs declaration.
    /// </summary>
    public class GoodsItem
    {
        public Guid Id { get; set; }

        /// <summary>HS (Harmonised System) customs code, e.g. "8471.30".</summary>
        public string CustomsCode { get; set; }

        public int Quantity { get; set; }

        public decimal UnitPrice { get; set; }
    }
}
