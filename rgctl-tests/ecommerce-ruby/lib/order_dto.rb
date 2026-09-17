module Trackable
end

class OrderDTO
  include Trackable
  attr_accessor :status

  def initialize(status)
    @status = status
  end

  def mark_processed
    @status = 'PROCESSED'
  end
end
