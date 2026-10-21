// Diagnostic wrapper for SimC76f8118; never used as the DPS reference
// executable. The pinned debug log prints a consumed tiered-node rank, not
// player_traits. Include its unchanged CLI driver with a renamed entry, reusing
// its built objects except sc_main.o. Retire when the official CLI exports
// lossless selected traits.
#define main simc_metadata_entry
#include "sc_main.cpp"
#undef main
#include "dbc/trait_data.hpp"
#include <fstream>

static void write_selected_traits(std::ostream &output,
                                  const player_t &player) {
  output << "[\n";
  bool first = true;
  for (const auto &[tree, entry, rank] : player.player_traits) {
    if (rank == 0)
      continue;
    const auto *trait = trait_data_t::find(entry, player.dbc->ptr);
    if (!trait || trait->id_trait_node_entry != entry ||
        rank > trait->max_ranks)
      throw std::runtime_error("Invalid selected native trait entry/rank");
    if (!first)
      output << ",\n";
    first = false;
    output << fmt::format(
        R"({{"tree":"{}","node_id":{},"entry_id":{},"ranks":{},"max_ranks":{}}})",
        util::talent_tree_string(tree), trait->id_node, entry, rank,
        trait->max_ranks);
  }
  output << "\n]\n";
}

int main(int argc, char **argv) {
  if (argc != 3) {
    fmt::print(stderr,
               "Usage: simc_selected_traits OUTPUT.json PROFILE.simc\n");
    return 2;
  }
  try {
    sim_t sim;
    const std::vector<std::string> arguments = {argv[2],
                                                "item_db_source=local",
                                                "iterations=1",
                                                "threads=1",
                                                "max_time=0.1",
                                                "fixed_time=1",
                                                "vary_combat_length=0",
                                                "optimal_raid=0",
                                                "output=/dev/null"};
    const int result = sim.simc_metadata_entry(arguments);
    if (result != 0)
      return result;
    if (sim.player_no_pet_list.size() != 1)
      throw std::runtime_error(
          "Selected-trait export requires exactly one player");
    std::ofstream output(argv[1]);
    output.exceptions(std::ios::badbit | std::ios::failbit);
    write_selected_traits(output, *sim.player_no_pet_list.front());
    return 0;
  } catch (const std::exception &error) {
    fmt::print(stderr, "Selected-trait export: {}\n", error.what());
    return 1;
  }
}
