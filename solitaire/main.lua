-- Solitaire Game using Love2D

-- Game state
local game = {
    cards = {},
    deck = {},
    tableau = {},  -- The seven columns of cards
    foundation = {},  -- The four piles for sorted cards
    waste = {},  -- Cards drawn from the deck
    dragging = nil,  -- Currently dragged card(s)
    dragOrigin = nil,  -- Where the dragged card(s) came from
    dragOffsetX = 0,
    dragOffsetY = 0
}

-- Card dimensions
local CARD_WIDTH = 80
local CARD_HEIGHT = 120
local CARD_SCALE = 1

-- Colors
local BACKGROUND_COLOR = {0, 0.5, 0, 1}  -- Green table

-- Initialize the game
function love.load()
    love.window.setTitle("Solitaire")
    love.window.setMode(800, 600)
    
    -- Initialize the game
    initializeGame()
    
    -- Load card images (placeholder for now)
    -- We'll implement this later
end

-- Initialize the game state
function initializeGame()
    -- Create and shuffle a deck of cards
    createDeck()
    shuffleDeck()
    
    -- Set up the tableau (the seven columns)
    setupTableau()
    
    -- Initialize the foundation piles
    for i = 1, 4 do
        game.foundation[i] = {}
    end
    
    -- Initialize the waste pile
    game.waste = {}
end

-- Create a standard deck of 52 cards
function createDeck()
    game.deck = {}
    local suits = {"hearts", "diamonds", "clubs", "spades"}
    local values = {"A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"}
    
    for _, suit in ipairs(suits) do
        for i, value in ipairs(values) do
            table.insert(game.deck, {
                suit = suit,
                value = value,
                rank = i,  -- Numerical rank (A=1, K=13)
                color = (suit == "hearts" or suit == "diamonds") and "red" or "black",
                faceUp = false,
                x = 0,
                y = 0
            })
        end
    end
end

-- Shuffle the deck
function shuffleDeck()
    for i = #game.deck, 2, -1 do
        local j = math.random(i)
        game.deck[i], game.deck[j] = game.deck[j], game.deck[i]
    end
end

-- Set up the tableau (the seven columns)
function setupTableau()
    game.tableau = {}
    
    for i = 1, 7 do
        game.tableau[i] = {}
        
        -- Deal i cards to column i
        for j = 1, i do
            local card = table.remove(game.deck)
            -- Only the top card is face up
            card.faceUp = (j == i)
            table.insert(game.tableau[i], card)
        end
    end
end

-- Update game state
function love.update(dt)
    -- We'll implement game logic here later
end

-- Draw the game
function love.draw()
    -- Set background color
    love.graphics.setBackgroundColor(BACKGROUND_COLOR)
    
    -- Draw the tableau (placeholder rectangles for now)
    drawTableau()
    
    -- Draw the foundation piles
    drawFoundation()
    
    -- Draw the deck and waste pile
    drawDeck()
    
    -- Draw the currently dragged card(s), if any
    if game.dragging then
        -- We'll implement this later
    end
end

-- Draw the tableau (the seven columns)
function drawTableau()
    local startX = 50
    local startY = 150
    local columnSpacing = CARD_WIDTH + 20
    
    for i, column in ipairs(game.tableau) do
        local x = startX + (i-1) * columnSpacing
        local y = startY
        
        -- Draw empty column placeholder
        love.graphics.setColor(0, 0.3, 0, 1)
        love.graphics.rectangle("line", x, y, CARD_WIDTH, CARD_HEIGHT)
        
        -- Draw cards in the column
        for j, card in ipairs(column) do
            -- Position the card
            card.x = x
            card.y = y + (j-1) * 30  -- Offset each card vertically
            
            -- Draw card placeholder
            if card.faceUp then
                love.graphics.setColor(1, 1, 1, 1)
            else
                love.graphics.setColor(0.2, 0.2, 0.8, 1)  -- Blue back
            end
            
            love.graphics.rectangle("fill", card.x, card.y, CARD_WIDTH, CARD_HEIGHT)
            love.graphics.setColor(0, 0, 0, 1)
            love.graphics.rectangle("line", card.x, card.y, CARD_WIDTH, CARD_HEIGHT)
            
            -- Draw card value and suit if face up
            if card.faceUp then
                love.graphics.setColor(card.color == "red" and {1, 0, 0, 1} or {0, 0, 0, 1})
                love.graphics.print(card.value .. " " .. card.suit:sub(1,1), card.x + 5, card.y + 5)
            end
        end
    end
end

-- Draw the foundation piles
function drawFoundation()
    local startX = 300
    local startY = 50
    local pileSpacing = CARD_WIDTH + 20
    
    for i = 1, 4 do
        local x = startX + (i-1) * pileSpacing
        local y = startY
        
        -- Draw empty foundation placeholder
        love.graphics.setColor(0, 0.3, 0, 1)
        love.graphics.rectangle("line", x, y, CARD_WIDTH, CARD_HEIGHT)
        
        -- Draw the top card if any
        if #game.foundation[i] > 0 then
            local card = game.foundation[i][#game.foundation[i]]
            -- We'll implement this later when we have actual cards in the foundation
        end
    end
end

-- Draw the deck and waste pile
function drawDeck()
    local deckX = 50
    local deckY = 50
    local wasteX = 150
    local wasteY = 50
    
    -- Draw deck placeholder
    love.graphics.setColor(0, 0.3, 0, 1)
    love.graphics.rectangle("line", deckX, deckY, CARD_WIDTH, CARD_HEIGHT)
    
    -- Draw waste pile placeholder
    love.graphics.rectangle("line", wasteX, wasteY, CARD_WIDTH, CARD_HEIGHT)
    
    -- Draw deck cards
    if #game.deck > 0 then
        love.graphics.setColor(0.2, 0.2, 0.8, 1)  -- Blue back
        love.graphics.rectangle("fill", deckX, deckY, CARD_WIDTH, CARD_HEIGHT)
        love.graphics.setColor(0, 0, 0, 1)
        love.graphics.rectangle("line", deckX, deckY, CARD_WIDTH, CARD_HEIGHT)
    end
    
    -- Draw top waste card if any
    if #game.waste > 0 then
        local card = game.waste[#game.waste]
        -- We'll implement this later when we have actual cards in the waste pile
    end
end

-- Handle mouse press
function love.mousepressed(x, y, button)
    -- We'll implement card dragging and game interactions later
end

-- Handle mouse release
function love.mousereleased(x, y, button)
    -- We'll implement card dropping and move validation later
end

-- Handle mouse movement
function love.mousemoved(x, y, dx, dy)
    -- We'll implement drag movement later
end

-- Handle key press
function love.keypressed(key)
    if key == "escape" then
        love.event.quit()
    elseif key == "r" then
        -- Reset the game
        initializeGame()
    end
end