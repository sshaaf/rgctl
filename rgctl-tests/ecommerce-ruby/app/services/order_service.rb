require_relative '../../lib/order_dto'

class OrderService
  def process(order)
    order.mark_processed
    order
  end

  def build(status)
    OrderDTO.new(status)
  end
end
