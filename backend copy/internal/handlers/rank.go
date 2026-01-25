package handlers

// RankTier represents the user's rank tier based on leaderboard position
type RankTier string

const (
	RankConqueror RankTier = "conqueror" // Top 1-5
	RankAce       RankTier = "ace"       // Top 6-10
	RankCrown     RankTier = "crown"      // Top 11-20
	RankDiamond   RankTier = "diamond"   // Top 21-50
	RankGold      RankTier = "gold"      // Top 51-100
	RankSilver    RankTier = "silver"     // Top 101-500
	RankBronze    RankTier = "bronze"    // Below 500 or no contributions
	RankTierUnranked RankTier = "unranked" // No contributions or not in ranking
)

// GetRankTier returns the rank tier based on leaderboard position
// Position is 1-indexed (1 = first place)
func GetRankTier(position int) RankTier {
	if position <= 0 {
		return RankBronze
	}
	if position <= 5 {
		return RankConqueror
	}
	if position <= 10 {
		return RankAce
	}
	if position <= 20 {
		return RankCrown
	}
	if position <= 50 {
		return RankDiamond
	}
	if position <= 100 {
		return RankGold
	}
	if position <= 500 {
		return RankSilver
	}
	return RankBronze
}

// GetRankTierDisplayName returns a human-readable name for the rank tier
func GetRankTierDisplayName(tier RankTier) string {
	switch tier {
	case RankConqueror:
		return "Conqueror"
	case RankAce:
		return "Ace"
	case RankCrown:
		return "Crown"
	case RankDiamond:
		return "Diamond"
	case RankGold:
		return "Gold"
	case RankSilver:
		return "Silver"
	case RankBronze:
		return "Bronze"
	case RankTierUnranked:
		return "Unranked"
	default:
		return "Bronze"
	}
}

// GetRankTierColor returns a color code for the rank tier (for UI)
func GetRankTierColor(tier RankTier) string {
	switch tier {
	case RankConqueror:
		return "#FFD700" // Gold
	case RankAce:
		return "#FF6B6B" // Red
	case RankCrown:
		return "#4ECDC4" // Teal
	case RankDiamond:
		return "#95E1D3" // Light Blue
	case RankGold:
		return "#F7DC6F" // Yellow
	case RankSilver:
		return "#C0C0C0" // Silver
	case RankBronze:
		return "#CD7F32" // Bronze
	case RankTierUnranked:
		return "#7a6b5a" // Gray
	default:
		return "#CD7F32"
	}
}

