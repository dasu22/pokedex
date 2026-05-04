"use strict";
const REGIONS = [
    { label: "Gen 1 – Kanto", region: "Kanto", caught: 0, total: 0 },
    { label: "Gen 2 – Johto", region: "Johto", caught: 0, total: 0 },
    { label: "Gen 3 – Hoenn", region: "Hoenn", caught: 0, total: 0 },
    { label: "Gen 4 – Sinnoh", region: "Sinnoh", caught: 0, total: 0 },
    { label: "Gen 5 – Unova", region: "Unova", caught: 0, total: 0 },
    { label: "Gen 6 – Kalos", region: "Kalos", caught: 0, total: 0 },
    { label: "Gen 7 – Alola", region: "Alola", caught: 0, total: 0 },
    { label: "Gen 8 – Galar/Hisui", region: "Galar/Hisui", caught: 0, total: 0 },
    { label: "Gen 9 – Paldea", region: "Paldea", caught: 0, total: 0 },
];
// ── State ──────────────────────────────────────────────────────────────
let allPokemon = [];
let sortCol = "number";
let sortDir = 1;
// ── Tab switching ──────────────────────────────────────────────────────
document.querySelectorAll(".tab-btn").forEach(btn => {
    btn.addEventListener("click", () => {
        document.querySelectorAll(".tab-btn").forEach(b => b.classList.remove("active"));
        document.querySelectorAll(".tab-panel").forEach(p => p.classList.remove("active"));
        btn.classList.add("active");
        document.getElementById(`tab-${btn.dataset.tab}`).classList.add("active");
    });
});
// ── Filters ───────────────────────────────────────────────────────────
const searchEl = document.getElementById("search");
const regionEl = document.getElementById("filter-region");
const typeEl = document.getElementById("filter-type");
const caughtEl = document.getElementById("filter-caught");
const resetBtn = document.getElementById("btn-reset-filters");
[searchEl, regionEl, typeEl, caughtEl].forEach(el => el.addEventListener("input", renderTable));
resetBtn.addEventListener("click", () => {
    searchEl.value = "";
    regionEl.value = "";
    typeEl.value = "";
    caughtEl.value = "";
    renderTable();
});
// ── Sort headers ───────────────────────────────────────────────────────
document.querySelectorAll("th.sortable").forEach(th => {
    th.addEventListener("click", () => {
        const col = th.dataset.col;
        if (sortCol === col) {
            sortDir = sortDir === 1 ? -1 : 1;
        }
        else {
            sortCol = col;
            sortDir = 1;
        }
        document.querySelectorAll("th.sortable").forEach(h => {
            h.classList.remove("sort-asc", "sort-desc");
        });
        th.classList.add(sortDir === 1 ? "sort-asc" : "sort-desc");
        renderTable();
    });
});
// ── Load data ──────────────────────────────────────────────────────────
async function loadData() {
    const res = await fetch("/api/pokemon");
    allPokemon = await res.json();
    populateFilters();
    renderTable();
    renderSummary();
}
function populateFilters() {
    const regions = [...new Set(allPokemon.map(p => p.region))];
    regions.forEach(r => {
        const opt = document.createElement("option");
        opt.value = r;
        opt.textContent = r;
        regionEl.appendChild(opt);
    });
    const types = [...new Set(allPokemon.flatMap(p => [p.type1, p.type2].filter(Boolean)))].sort();
    types.forEach(t => {
        const opt = document.createElement("option");
        opt.value = t;
        opt.textContent = t;
        typeEl.appendChild(opt);
    });
}
// ── Render Pokémon table ───────────────────────────────────────────────
function renderTable() {
    const search = searchEl.value.toLowerCase();
    const region = regionEl.value;
    const type = typeEl.value;
    const caught = caughtEl.value;
    let filtered = allPokemon.filter(p => {
        if (search && !p.name.toLowerCase().includes(search))
            return false;
        if (region && p.region !== region)
            return false;
        if (type && p.type1 !== type && p.type2 !== type)
            return false;
        if (caught === "caught" && !p.caught)
            return false;
        if (caught === "missing" && p.caught)
            return false;
        return true;
    });
    filtered.sort((a, b) => {
        const av = a[sortCol];
        const bv = b[sortCol];
        if (typeof av === "number" && typeof bv === "number")
            return (av - bv) * sortDir;
        return String(av).localeCompare(String(bv)) * sortDir;
    });
    const tbody = document.getElementById("pokemon-body");
    tbody.innerHTML = "";
    filtered.forEach(p => {
        const tr = document.createElement("tr");
        tr.className = p.caught ? "is-caught" : "";
        tr.dataset.id = String(p.number);
        tr.innerHTML = `
      <td class="col-caught">
        <button class="catch-btn ${p.caught ? "caught" : "missing"}"
                aria-label="${p.caught ? "Mark as missing" : "Mark as caught"}"
                data-id="${p.number}">
          ${p.caught ? "✓" : ""}
        </button>
      </td>
      <td class="col-num">${p.number}</td>
      <td>${p.name}</td>
      <td>${typeBadge(p.type1)}</td>
      <td>${p.type2 ? typeBadge(p.type2) : ""}</td>
      <td class="col-gen">${p.generation}</td>
      <td>${p.region}</td>
      <td class="col-notes">${p.notes}</td>
    `;
        tbody.appendChild(tr);
    });
    // Attach toggle listeners
    tbody.querySelectorAll(".catch-btn").forEach(btn => {
        btn.addEventListener("click", () => toggleCaught(Number(btn.dataset.id), btn));
    });
    const countEl = document.getElementById("pokemon-count");
    countEl.textContent = `Showing ${filtered.length} of ${allPokemon.length} Pokémon`;
}
function typeBadge(type) {
    return `<span class="type-badge type-${type}">${type}</span>`;
}
// ── Toggle caught ──────────────────────────────────────────────────────
async function toggleCaught(id, btn) {
    btn.classList.add("loading");
    try {
        const res = await fetch(`/api/pokemon/${id}/toggle`, { method: "POST" });
        if (!res.ok)
            throw new Error("Toggle failed");
        const updated = await res.json();
        const idx = allPokemon.findIndex(p => p.number === id);
        if (idx !== -1)
            allPokemon[idx] = updated;
        renderTable();
        renderSummary();
    }
    catch {
        btn.classList.remove("loading");
    }
}
// ── Summary tab ────────────────────────────────────────────────────────
function renderSummary() {
    const totals = REGIONS.map(r => ({
        ...r,
        caught: allPokemon.filter(p => p.region === r.region && p.caught).length,
        total: allPokemon.filter(p => p.region === r.region).length,
    }));
    const grandCaught = totals.reduce((s, r) => s + r.caught, 0);
    const grandTotal = totals.reduce((s, r) => s + r.total, 0);
    const pct = grandTotal > 0 ? (grandCaught / grandTotal) * 100 : 0;
    const overallBar = document.getElementById("overall-bar");
    const overallLabel = document.getElementById("overall-label");
    overallBar.style.width = `${pct.toFixed(1)}%`;
    overallLabel.textContent =
        `${grandCaught} / ${grandTotal} caught — ${pct.toFixed(2)}% complete — ${grandTotal - grandCaught} remaining`;
    const grid = document.getElementById("summary-grid");
    grid.innerHTML = "";
    totals.forEach(r => {
        const rPct = r.total > 0 ? (r.caught / r.total) * 100 : 0;
        const card = document.createElement("div");
        card.className = "summary-card";
        card.innerHTML = `
      <h3>${r.label}</h3>
      <div class="summary-stats">
        <span class="stat-caught">${r.caught}</span>
        <span class="stat-total">/ ${r.total}</span>
        <span class="stat-remaining">${r.total - r.caught} remaining</span>
      </div>
      <div class="region-bar-wrap">
        <div class="region-bar" style="width:${rPct.toFixed(1)}%"></div>
      </div>
      <div class="region-pct">${rPct.toFixed(1)}%</div>
    `;
        grid.appendChild(card);
    });
}
// ── Init ───────────────────────────────────────────────────────────────
loadData();
