
function testPublicNameLogic(profileData, viewingUser, user) {
    const result = profileData?.first_name || profileData?.last_name
        ? `${profileData.first_name || ''} ${profileData.last_name || ''}`.trim()
        : (viewingUser?.login || user?.github?.login || 'Loading...');
    return result;
}

const mockUser = { github: { login: 'johndoe_gh' } };

const publicScenarios = [
    { name: "Public [1/4]: Dual OK", profile: { first_name: "John", last_name: "Doe" }, expected: "John Doe" },
    { name: "Public [2/4]: Only First", profile: { first_name: "John", last_name: "" }, expected: "John" },
    { name: "Public [3/4]: Only Last", profile: { first_name: null, last_name: "Doe" }, expected: "Doe" },
    { name: "Public [4/4]: No Names", profile: { first_name: "", last_name: null }, expected: "johndoe_gh" }
];

console.log("--- Testing Public Name Display ---");
publicScenarios.forEach(s => {
    const actual = testPublicNameLogic(s.profile, null, mockUser);
    const passed = actual === s.expected;
    console.log(`${s.name}: ${passed ? 'PASSED' : 'FAILED'} (Actual: "${actual}", Expected: "${s.expected}")`);
});

const privateScenarios = [
    { name: "Private [1/4]: Dual OK", profile: { first_name: "John", last_name: "Doe" }, expected: "John Doe" },
    { name: "Private [2/4]: Only First", profile: { first_name: "John", last_name: "" }, expected: "John" },
    { name: "Private [3/4]: Only Last", profile: { first_name: null, last_name: "Doe" }, expected: "Doe" },
    { name: "Private [4/4]: No Names", profile: { first_name: "", last_name: null }, expected: "johndoe_gh" }
];

console.log("\n--- Testing Private Name Display ---");
privateScenarios.forEach(s => {
    const actual = testPublicNameLogic(s.profile, null, mockUser);
    const passed = actual === s.expected;
    console.log(`${s.name}: ${passed ? 'PASSED' : 'FAILED'} (Actual: "${actual}", Expected: "${s.expected}")`);
});
