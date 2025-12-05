-- main.lua - Klondike Solitaire Game
-- A classic solitaire card game implementation using LÖVE2D

-- Constants
local CARD_WIDTH = 80
local CARD_HEIGHT = 120
local CARD_SCALE = 0.8
local TABLEAU_X = 50
local TABLEAU_Y = 200
local TABLEAU_OFFSET_X = 90
local FOUNDATION_X = 320
local FOUNDATION_Y = 50
local FOUNDATION_OFFSET_X = 90
local STOCK_X = 50
local STOCK_Y = 50
local WASTE_X = 150
local WASTE_Y = 50
local CARD_OFFSET_Y = 30
local FACE_DOWN_OFFSET_Y = 15

-- Game state
local deck = {}
local tableau = {}
local foundations = {}
local stock = {}
local waste = {}
local dragging = {active = false, cards = {}, source = nil, offsetX = 0, offsetY = 0}
local score = 0
local moves = 0
local gameWon = false
local fonts = {}
local cardImages = {}
local backImage

-- Initialize the game
function love.load()
    -- Set random seed
    math.randomseed(os.time())
    
    -- Load fonts
    fonts.large = love.graphics.newFont(24)
    fonts.medium = love.graphics.newFont(18)
    fonts.small = love.graphics.newFont(14)
    
    -- Load card images
    loadCardImages()
    
    -- Initialize game
    initializeGame()
end

-- Load card images
function loadCardImages()
    local suits = {"hearts", "diamonds", "clubs", "spades"}
    local values = {"ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"}
    
    cardImages = {}
    for _, suit in ipairs(suits) do
        cardImages[suit] = {}
        for _, value in ipairs(values) do
            local filename = "cards/" .. value .. "_of_" .. suit .. ".png"
            -- Note: In a real implementation, you would need actual card images
            -- For this example, we'll create placeholder colored rectangles
            cardImages[suit][value] = {suit = suit, value = value}
        end
    end
    
    -- Card back image
    backImage = {back = true}
end

-- Initialize a new game
function initializeGame()
    -- Create a standard deck of cards
    createDeck()
    
    -- Shuffle the deck
    shuffleDeck()
    
    -- Initialize tableau piles
    initializeTableau()
    
    -- Initialize foundation piles
    initializeFoundations()
    
    -- Remaining cards go to stock
    stock = {}
    for i = #deck, 1, -1 do
        table.insert(stock, table.remove(deck, i))
    end
    
    -- Initialize waste pile
    waste = {}
    
    -- Reset game state
    score = 0
    moves = 0
    gameWon = false
    dragging = {active = false, cards = {}, source = nil, offsetX = 0, offsetY = 0}
end

-- Create a standard deck of cards
function createDeck()
    deck = {}
    local suits = {"hearts", "diamonds", "clubs", "spades"}
    local values = {"ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"}
    local valueMap = {
        ace = 1, ["2"] = 2, ["3"] = 3, ["4"] = 4, ["5"] = 5, ["6"] = 6, ["7"] = 7,
        ["8"] = 8, ["9"] = 9, ["10"] = 10, jack = 11, queen = 12, king = 13
    }
    
    for _, suit in ipairs(suits) do
        for _, value in ipairs(values) do
            local card = {
                suit = suit,
                value = value,
                numValue = valueMap[value],
                faceUp = false,
                color = (suit == "hearts" or suit == "diamonds") and "red" or "black"
            }
            table.insert(deck, card)
        end
    end
end

-- Shuffle the deck
function shuffleDeck()
    for i = #deck, 2, -1 do
        local j = math.random(i)
        deck[i], deck[j] = deck[j], deck[i]
    end
end

-- Initialize tableau piles
function initializeTableau()
    tableau = {}
    for i = 1, 7 do
        tableau[i] = {}
        for j = 1, i do
            local card = table.remove(deck)
            card.faceUp = (j == i)  -- Only the top card is face up
            table.insert(tableau[i], card)
        end
    end
end

-- Initialize foundation piles
function initializeFoundations()
    foundations = {}
    for i = 1, 4 do
        foundations[i] = {}
    end
end

-- Draw the game
function love.draw()
    -- Set background color
    love.graphics.setBackgroundColor(0, 0.5, 0, 1)
    
    -- Draw tableau piles
    drawTableau()
    
    -- Draw foundation piles
    drawFoundations()
    
    -- Draw stock and waste piles
    drawStockAndWaste()
    
    -- Draw dragging cards
    if dragging.active then
        drawDraggingCards()
    end
    
    -- Draw score and moves
    drawUI()
    
    -- Draw win message if game is won
    if gameWon then
        drawWinMessage()
    end
end

-- Draw tableau piles
function drawTableau()
    for i = 1, 7 do
        -- Draw empty pile placeholder
        love.graphics.setColor(0, 0.3, 0, 0.5)
        love.graphics.rectangle("fill", TABLEAU_X + (i-1) * TABLEAU_OFFSET_X, TABLEAU_Y, 
                               CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
        love.graphics.setColor(1, 1, 1, 0.2)
        love.graphics.rectangle("line", TABLEAU_X + (i-1) * TABLEAU_OFFSET_X, TABLEAU_Y, 
                               CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
        
        -- Draw cards in the pile
        for j, card in ipairs(tableau[i]) do
            if not (dragging.active and dragging.source == "tableau" and dragging.pileIndex == i and j >= dragging.cardIndex) then
                drawCard(card, TABLEAU_X + (i-1) * TABLEAU_OFFSET_X, 
                        TABLEAU_Y + (j-1) * (card.faceUp and CARD_OFFSET_Y or FACE_DOWN_OFFSET_Y))
            end
        end
    end
end

-- Draw foundation piles
function drawFoundations()
    for i = 1, 4 do
        -- Draw empty pile placeholder
        love.graphics.setColor(0, 0.3, 0, 0.5)
        love.graphics.rectangle("fill", FOUNDATION_X + (i-1) * FOUNDATION_OFFSET_X, FOUNDATION_Y, 
                               CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
        love.graphics.setColor(1, 1, 1, 0.2)
        love.graphics.rectangle("line", FOUNDATION_X + (i-1) * FOUNDATION_OFFSET_X, FOUNDATION_Y, 
                               CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
        
        -- Draw top card if any
        if #foundations[i] > 0 then
            local card = foundations[i][#foundations[i]]
            drawCard(card, FOUNDATION_X + (i-1) * FOUNDATION_OFFSET_X, FOUNDATION_Y)
        end
    end
end

-- Draw stock and waste piles
function drawStockAndWaste()
    -- Draw stock pile
    love.graphics.setColor(0, 0.3, 0, 0.5)
    love.graphics.rectangle("fill", STOCK_X, STOCK_Y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
    love.graphics.setColor(1, 1, 1, 0.2)
    love.graphics.rectangle("line", STOCK_X, STOCK_Y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
    
    if #stock > 0 then
        drawCard({faceUp = false}, STOCK_X, STOCK_Y)
    end
    
    -- Draw waste pile
    love.graphics.setColor(0, 0.3, 0, 0.5)
    love.graphics.rectangle("fill", WASTE_X, WASTE_Y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
    love.graphics.setColor(1, 1, 1, 0.2)
    love.graphics.rectangle("line", WASTE_X, WASTE_Y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
    
    -- Draw up to 3 waste cards with slight offset
    local startIdx = math.max(1, #waste - 2)
    for i = startIdx, #waste do
        local offsetX = (i - startIdx) * 20
        if not (dragging.active and dragging.source == "waste" and i == #waste) then
            drawCard(waste[i], WASTE_X + offsetX, WASTE_Y)
        end
    end
end

-- Draw a single card
function drawCard(card, x, y)
    if card.faceUp then
        -- Draw face up card
        if card.color == "red" then
            love.graphics.setColor(0.9, 0.2, 0.2, 1)
        else
            love.graphics.setColor(0.1, 0.1, 0.1, 1)
        end
        love.graphics.rectangle("fill", x, y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
        love.graphics.setColor(1, 1, 1, 1)
        love.graphics.rectangle("line", x, y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
        
        -- Draw card value and suit
        love.graphics.setFont(fonts.medium)
        love.graphics.setColor(1, 1, 1, 1)
        love.graphics.print(card.value, x + 5, y + 5)
        love.graphics.print(card.suit:sub(1, 1):upper(), x + 5, y + 25)
    else
        -- Draw face down card
        love.graphics.setColor(0.2, 0.2, 0.8, 1)
        love.graphics.rectangle("fill", x, y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
        love.graphics.setColor(1, 1, 1, 1)
        love.graphics.rectangle("line", x, y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE, 5, 5)
        
        -- Draw pattern on back
        love.graphics.setColor(0.1, 0.1, 0.7, 1)
        love.graphics.rectangle("fill", x + 10, y + 10, 
                               (CARD_WIDTH * CARD_SCALE) - 20, (CARD_HEIGHT * CARD_SCALE) - 20, 3, 3)
    end
end

-- Draw cards being dragged
function drawDraggingCards()
    local mouseX, mouseY = love.mouse.getPosition()
    local x = mouseX - dragging.offsetX
    local y = mouseY - dragging.offsetY
    
    for i, card in ipairs(dragging.cards) do
        drawCard(card, x, y + (i-1) * CARD_OFFSET_Y)
    end
end

-- Draw UI elements (score, moves)
function drawUI()
    love.graphics.setFont(fonts.medium)
    love.graphics.setColor(1, 1, 1, 1)
    love.graphics.print("Score: " .. score, 650, 50)
    love.graphics.print("Moves: " .. moves, 650, 80)
    
    -- Draw restart button
    love.graphics.setColor(0.3, 0.3, 0.8, 1)
    love.graphics.rectangle("fill", 650, 120, 100, 30, 5, 5)
    love.graphics.setColor(1, 1, 1, 1)
    love.graphics.print("Restart", 670, 125)
end

-- Draw win message
function drawWinMessage()
    love.graphics.setColor(0, 0, 0, 0.7)
    love.graphics.rectangle("fill", 0, 0, love.graphics.getWidth(), love.graphics.getHeight())
    
    love.graphics.setFont(fonts.large)
    love.graphics.setColor(1, 1, 0, 1)
    love.graphics.printf("You Win!", 0, 300, love.graphics.getWidth(), "center")
    
    love.graphics.setFont(fonts.medium)
    love.graphics.setColor(1, 1, 1, 1)
    love.graphics.printf("Score: " .. score, 0, 350, love.graphics.getWidth(), "center")
    love.graphics.printf("Moves: " .. moves, 0, 380, love.graphics.getWidth(), "center")
    love.graphics.printf("Click anywhere to play again", 0, 430, love.graphics.getWidth(), "center")
end

-- Update game state
function love.update(dt)
    -- Check for win condition
    checkWinCondition()
end

-- Check if the game is won
function checkWinCondition()
    if not gameWon then
        local allCardsInFoundations = true
        for i = 1, 4 do
            if #foundations[i] < 13 then
                allCardsInFoundations = false
                break
            end
        end
        
        if allCardsInFoundations then
            gameWon = true
        end
    end
end

-- Handle mouse press
function love.mousepressed(x, y, button)
    if button == 1 then  -- Left mouse button
        if gameWon then
            -- Restart game if won
            initializeGame()
            return
        end
        
        -- Check if restart button was clicked
        if x >= 650 and x <= 750 and y >= 120 and y <= 150 then
            initializeGame()
            return
        end
        
        -- Check if stock was clicked
        if isPointInRect(x, y, STOCK_X, STOCK_Y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE) then
            handleStockClick()
            return
        end
        
        -- Check if waste was clicked
        if isPointInRect(x, y, WASTE_X, WASTE_Y, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE) and #waste > 0 then
            startDraggingFromWaste(x, y)
            return
        end
        
        -- Check if tableau was clicked
        for i = 1, 7 do
            local pileX = TABLEAU_X + (i-1) * TABLEAU_OFFSET_X
            local pileY = TABLEAU_Y
            local pileHeight = CARD_HEIGHT * CARD_SCALE
            
            if #tableau[i] > 0 then
                pileHeight = pileHeight + (#tableau[i] - 1) * CARD_OFFSET_Y
            end
            
            if isPointInRect(x, y, pileX, pileY, CARD_WIDTH * CARD_SCALE, pileHeight) then
                startDraggingFromTableau(i, x, y)
                return
            end
        end
        
        -- Check if foundation was clicked
        for i = 1, 4 do
            local pileX = FOUNDATION_X + (i-1) * FOUNDATION_OFFSET_X
            local pileY = FOUNDATION_Y
            
            if isPointInRect(x, y, pileX, pileY, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE) and #foundations[i] > 0 then
                startDraggingFromFoundation(i, x, y)
                return
            end
        end
    end
end

-- Handle mouse release
function love.mousereleased(x, y, button)
    if button == 1 and dragging.active then  -- Left mouse button
        -- Try to place the dragged cards
        local placed = false
        
        -- Check if dropping on tableau
        for i = 1, 7 do
            local pileX = TABLEAU_X + (i-1) * TABLEAU_OFFSET_X
            local pileY = TABLEAU_Y
            
            if isPointInRect(x, y, pileX, pileY, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE + 200) then
                placed = tryPlaceOnTableau(i)
                break
            end
        end
        
        -- Check if dropping on foundation
        if not placed then
            for i = 1, 4 do
                local pileX = FOUNDATION_X + (i-1) * FOUNDATION_OFFSET_X
                local pileY = FOUNDATION_Y
                
                if isPointInRect(x, y, pileX, pileY, CARD_WIDTH * CARD_SCALE, CARD_HEIGHT * CARD_SCALE) then
                    placed = tryPlaceOnFoundation(i)
                    break
                end
            end
        end
        
        -- If not placed, return cards to original position
        if not placed then
            returnDraggedCards()
        end
        
        -- Reset dragging state
        dragging.active = false
        dragging.cards = {}
        dragging.source = nil
    end
end

-- Handle stock click
function handleStockClick()
    if #stock > 0 then
        -- Deal 3 cards from stock to waste
        for i = 1, math.min(3, #stock) do
            local card = table.remove(stock)
            card.faceUp = true
            table.insert(waste, card)
        end
        moves = moves + 1
    else
        -- Recycle waste back to stock
        while #waste > 0 do
            local card = table.remove(waste)
            card.faceUp = false
            table.insert(stock, card)
        end
        moves = moves + 1
    end
end

-- Start dragging from waste
function startDraggingFromWaste(x, y)
    if #waste > 0 then
        local card = waste[#waste]
        if card.faceUp then
            dragging.active = true
            dragging.cards = {table.remove(waste)}
            dragging.source = "waste"
            
            -- Calculate offset for smooth dragging
            local cardX = WASTE_X + (#waste > 0 and (#waste - 1) * 20 or 0)
            local cardY = WASTE_Y
            dragging.offsetX = x - cardX
            dragging.offsetY = y - cardY
        end
    end
end

-- Start dragging from tableau
function startDraggingFromTableau(pileIndex, x, y)
    local pile = tableau[pileIndex]
    if #pile == 0 then return end
    
    -- Find which card was clicked
    local cardIndex = 1
    for i = 1, #pile do
        local cardY = TABLEAU_Y + (i-1) * (pile[i].faceUp and CARD_OFFSET_Y or FACE_DOWN_OFFSET_Y)
        local nextCardY = i < #pile and (TABLEAU_Y + i * (pile[i+1].faceUp and CARD_OFFSET_Y or FACE_DOWN_OFFSET_Y)) or (cardY + CARD_HEIGHT * CARD_SCALE)
        
        if y >= cardY and y <= nextCardY then
            cardIndex = i
            break
        end
    end
    
    -- Can only drag face up cards
    if not pile[cardIndex].faceUp then return end
    
    -- Collect all cards from the clicked one to the end
    dragging.active = true
    dragging.cards = {}
    dragging.source = "tableau"
    dragging.pileIndex = pileIndex
    dragging.cardIndex = cardIndex
    
    for i = cardIndex, #pile do
        table.insert(dragging.cards, pile[i])
    end
    
    -- Remove dragged cards from the tableau
    for i = #pile, cardIndex, -1 do
        table.remove(pile, i)
    end
    
    -- Turn over the new top card if needed
    if #pile > 0 and not pile[#pile].faceUp then
        pile[#pile].faceUp = true
        score = score + 5  -- Score for revealing a card
    end
    
    -- Calculate offset for smooth dragging
    local cardX = TABLEAU_X + (pileIndex-1) * TABLEAU_OFFSET_X
    local cardY = TABLEAU_Y + (cardIndex-1) * CARD_OFFSET_Y
    dragging.offsetX = x - cardX
    dragging.offsetY = y - cardY
end

-- Start dragging from foundation
function startDraggingFromFoundation(pileIndex, x, y)
    local pile = foundations[pileIndex]
    if #pile == 0 then return end
    
    -- Can only drag the top card from foundation
    dragging.active = true
    dragging.cards = {table.remove(pile)}
    dragging.source = "foundation"
    dragging.pileIndex = pileIndex
    
    -- Calculate offset for smooth dragging
    local cardX = FOUNDATION_X + (pileIndex-1) * FOUNDATION_OFFSET_X
    local cardY = FOUNDATION_Y
    dragging.offsetX = x - cardX
    dragging.offsetY = y - cardY
end

-- Try to place cards on tableau
function tryPlaceOnTableau(pileIndex)
    local pile = tableau[pileIndex]
    local draggedCard = dragging.cards[1]
    
    -- Check if valid move
    if #pile == 0 then
        -- Empty pile can only accept Kings
        if draggedCard.numValue == 13 then
            -- Place all dragged cards
            for _, card in ipairs(dragging.cards) do
                table.insert(pile, card)
            end
            moves = moves + 1
            return true
        end
    else
        local topCard = pile[#pile]
        -- Cards must alternate colors and be in descending order
        if topCard.faceUp and topCard.color ~= draggedCard.color and topCard.numValue == draggedCard.numValue + 1 then
            -- Place all dragged cards
            for _, card in ipairs(dragging.cards) do
                table.insert(pile, card)
            end
            moves = moves + 1
            return true
        end
    end
    
    return false
end

-- Try to place card on foundation
function tryPlaceOnFoundation(pileIndex)
    -- Can only place one card at a time on foundation
    if #dragging.cards > 1 then
        return false
    end
    
    local pile = foundations[pileIndex]
    local card = dragging.cards[1]
    
    -- Check if valid move
    if #pile == 0 then
        -- Empty foundation can only accept Aces
        if card.numValue == 1 then
            table.insert(pile, card)
            score = score + 10  -- Score for placing on foundation
            moves = moves + 1
            return true
        end
    else
        local topCard = pile[#pile]
        -- Cards must be same suit and in ascending order
        if card.suit == topCard.suit and card.numValue == topCard.numValue + 1 then
            table.insert(pile, card)
            score = score + 10  -- Score for placing on foundation
            moves = moves + 1
            return true
        end
    end
    
    return false
end

-- Return dragged cards to their original position
function returnDraggedCards()
    if dragging.source == "tableau" then
        local pile = tableau[dragging.pileIndex]
        for _, card in ipairs(dragging.cards) do
            table.insert(pile, card)
        end
    elseif dragging.source == "waste" then
        for _, card in ipairs(dragging.cards) do
            table.insert(waste, card)
        end
    elseif dragging.source == "foundation" then
        local pile = foundations[dragging.pileIndex]
        for _, card in ipairs(dragging.cards) do
            table.insert(pile, card)
        end
    end
end

-- Helper function to check if a point is inside a rectangle
function isPointInRect(x, y, rectX, rectY, rectWidth, rectHeight)
    return x >= rectX and x <= rectX + rectWidth and y >= rectY and y <= rectY + rectHeight
end

-- Handle keyboard input
function love.keypressed(key)
    if key == "escape" then
        love.event.quit()
    elseif key == "r" then
        initializeGame()
    end
end